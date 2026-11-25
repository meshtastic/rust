use bytes::{Buf, BufMut, BytesMut};
use prost::Message;
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    connections::wrappers::encoded_data::EncodedToRadioPacket,
    errors_internal::Error,
    protobufs::{FromRadio, ToRadio},
};

const HEADER_MAGIC: [u8; 2] = [0x94, 0xc3];
const HEADER_SIZE: usize = 4;
const MAX_PACKET_SIZE: usize = 512;
const MAX_LOG_LINE_SIZE: usize = 4096;

enum LogOrMagic {
    Newline(usize),
    Magic(usize),
    None,
}

/// Scans `src` in a single pass for the first log newline (`\n`) or packet
/// magic header (`[0x94, 0xc3]`).
fn find_log_or_magic(src: &[u8]) -> LogOrMagic {
    for (i, &b) in src.iter().enumerate() {
        if b == b'\n' {
            return LogOrMagic::Newline(i);
        }
        if b == HEADER_MAGIC[0] && src.get(i + 1) == Some(&HEADER_MAGIC[1]) {
            return LogOrMagic::Magic(i);
        }
    }
    LogOrMagic::None
}

/// Returns `true` if `src[offset..]` begins with a complete frame whose
/// payload decodes as [`FromRadio`].
fn is_valid_frame_at(src: &[u8], offset: usize) -> bool {
    let Some(header) = src.get(offset..offset + HEADER_SIZE) else {
        return false;
    };
    let len = u16::from_be_bytes([header[2], header[3]]) as usize;
    if len > MAX_PACKET_SIZE {
        return false;
    }
    let Some(payload) = src.get(offset + HEADER_SIZE..offset + HEADER_SIZE + len) else {
        return false;
    };
    FromRadio::decode(payload).is_ok()
}

/// Searches `src[start..end]` for the earliest `[0x94, 0xc3]` header that is either
/// a complete valid [`FromRadio`] frame or a plausible incomplete frame awaiting more bytes.
fn find_plausible_frame_from(src: &[u8], start: usize, end: usize) -> Option<usize> {
    let mut idx = start;
    while idx < end {
        let Some(rel) = src[idx..end]
            .windows(2)
            .position(|window| window == HEADER_MAGIC)
        else {
            break;
        };
        let candidate = idx + rel;
        let Some(header) = src.get(candidate..candidate + HEADER_SIZE) else {
            return Some(candidate);
        };
        let len = u16::from_be_bytes([header[2], header[3]]) as usize;
        if len <= MAX_PACKET_SIZE
            && (src.len() < candidate + HEADER_SIZE + len || is_valid_frame_at(src, candidate))
        {
            return Some(candidate);
        }
        idx = candidate + HEADER_MAGIC.len();
    }
    None
}

/// Computes how many bytes to skip when a header at index 0 declares an
/// oversized payload, preserving any `0x94` or `[0x94, 0xc3]` in the length bytes.
fn oversized_header_advance(src: &[u8]) -> usize {
    if src[HEADER_MAGIC.len()..HEADER_SIZE] == HEADER_MAGIC {
        HEADER_MAGIC.len()
    } else if src[HEADER_SIZE - 1] == HEADER_MAGIC[0] {
        HEADER_SIZE - 1
    } else {
        HEADER_SIZE
    }
}

/// Validates payload size and writes the 4-byte wire header into `dst`.
fn write_frame_header(dst: &mut BytesMut, payload_len: usize) -> Result<(), Error> {
    if payload_len > MAX_PACKET_SIZE {
        return Err(Error::InvalidaDataSize {
            data_length: payload_len,
        });
    }
    dst.reserve(HEADER_SIZE + payload_len);
    dst.put_slice(&HEADER_MAGIC);
    dst.put_u16(payload_len as u16);
    Ok(())
}

/// A decoded frame emitted by [`MeshtasticCodec`].
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A decoded [`FromRadio`] protobuf packet.
    Packet(FromRadio),
    /// Non-framed console/debug output line emitted by the radio firmware.
    RadioLog(String),
}

/// A `tokio_util::codec` implementation for the Meshtastic serial/TCP wire protocol.
///
/// Each framed packet starts with `[0x94, 0xc3, len_msb, len_lsb]` followed by
/// `(len_msb << 8) | len_lsb` bytes of protobuf-encoded payload. Bytes appearing
/// outside valid frames are buffered line-by-line and emitted as [`Frame::RadioLog`].
#[derive(Debug, Default)]
pub struct MeshtasticCodec {
    log_line: Vec<u8>,
}

impl MeshtasticCodec {
    /// Creates a new [`MeshtasticCodec`] instance.
    pub fn new() -> Self {
        Self::default()
    }

    fn take_log_frame(&mut self) -> Option<Frame> {
        if self.log_line.is_empty() {
            return None;
        }
        let line = String::from_utf8_lossy(&self.log_line).into_owned();
        self.log_line.clear();
        Some(Frame::RadioLog(line))
    }

    /// Consumes up to `max_len` non-packet bytes from the front of `src`,
    /// buffering into `self.log_line` and returning a [`Frame::RadioLog`] as soon
    /// as a newline (`\n`) is encountered or [`MAX_LOG_LINE_SIZE`] is reached.
    fn consume_log_bytes(&mut self, src: &mut BytesMut, max_len: usize) -> Option<Frame> {
        let mut consumed = 0;
        let mut completed_line = None;

        for &b in &src[..max_len] {
            consumed += 1;
            if b == b'\n' {
                if let Some(frame) = self.take_log_frame() {
                    completed_line = Some(frame);
                    break;
                }
            } else if b != b'\r' {
                self.log_line.push(b);
                if self.log_line.len() >= MAX_LOG_LINE_SIZE {
                    completed_line = self.take_log_frame();
                    break;
                }
            }
        }

        src.advance(consumed);
        completed_line
    }
}

impl Decoder for MeshtasticCodec {
    type Item = Frame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            if src.is_empty() {
                return Ok(None);
            }

            match find_log_or_magic(src) {
                LogOrMagic::Newline(nl_pos) => {
                    if let Some(frame) = self.consume_log_bytes(src, nl_pos + 1) {
                        return Ok(Some(frame));
                    }
                    continue;
                }
                LogOrMagic::Magic(pos) => {
                    if pos > 0 {
                        if let Some(frame) = self.consume_log_bytes(src, pos) {
                            return Ok(Some(frame));
                        }
                    }
                }
                LogOrMagic::None => {
                    // If the buffer ends with 0x94, keep that byte in case 0xc3 arrives next.
                    let ends_with_magic_start = src.last() == Some(&HEADER_MAGIC[0]);
                    let log_len = src.len() - usize::from(ends_with_magic_start);

                    if log_len == 0 {
                        return Ok(None);
                    }

                    if let Some(frame) = self.consume_log_bytes(src, log_len) {
                        return Ok(Some(frame));
                    }
                    continue;
                }
            }

            // Packet header [0x94, 0xc3] is at index 0; need at least 4 bytes for the header.
            if src.len() < HEADER_SIZE {
                return Ok(None);
            }

            let packet_len = u16::from_be_bytes([src[2], src[3]]) as usize;
            if packet_len > MAX_PACKET_SIZE {
                log::warn!(
                    "Packet length {packet_len} exceeds maximum allowed size {MAX_PACKET_SIZE}, skipping header"
                );
                src.advance(oversized_header_advance(src));
                continue;
            }

            let total_frame_len = HEADER_SIZE + packet_len;
            if src.len() < total_frame_len {
                src.reserve(total_frame_len - src.len());
                return Ok(None);
            }

            if let Ok(packet) = FromRadio::decode(&src[HEADER_SIZE..total_frame_len]) {
                src.advance(total_frame_len);
                return Ok(Some(Frame::Packet(packet)));
            }

            let search_end = (total_frame_len + 1).min(src.len());
            if let Some(next_header_idx) = find_plausible_frame_from(src, HEADER_SIZE, search_end) {
                log::warn!(
                    "Detected malformed packet with next packet starting at index {next_header_idx}, purging malformed packet"
                );
                src.advance(next_header_idx);
            } else {
                let advance_len = if src.len() == total_frame_len
                    && src[total_frame_len - 1] == HEADER_MAGIC[0]
                {
                    total_frame_len - 1
                } else {
                    total_frame_len
                };
                log::warn!("Failed to decode FromRadio packet, discarding malformed frame");
                src.advance(advance_len);
            }
        }
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(frame) = self.decode(src)? {
            return Ok(Some(frame));
        }

        if !src.is_empty() {
            log::debug!("Discarding {} trailing incomplete bytes at EOF", src.len());
            src.clear();
        }

        Ok(self.take_log_frame())
    }
}

impl Encoder<ToRadio> for MeshtasticCodec {
    type Error = Error;

    fn encode(&mut self, item: ToRadio, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let len = item.encoded_len();
        write_frame_header(dst, len)?;
        item.encode_raw(dst);
        Ok(())
    }
}

impl Encoder<EncodedToRadioPacket> for MeshtasticCodec {
    type Error = Error;

    fn encode(
        &mut self,
        item: EncodedToRadioPacket,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        let data = item.data();
        write_frame_header(dst, data.len())?;
        dst.put_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{protobufs, utils_internal::format_data_packet};
    use prost::Message;

    fn mock_encoded_from_radio_packet(
        payload_variant: protobufs::from_radio::PayloadVariant,
        id: impl Into<Option<u32>>,
    ) -> (protobufs::FromRadio, Vec<u8>) {
        let packet_id = id.into().unwrap_or_else(rand::random);

        let packet = protobufs::FromRadio {
            id: packet_id,
            payload_variant: Some(payload_variant),
        };

        (packet.clone(), packet.encode_to_vec())
    }

    /// Helper that drains all currently decodable frames from `src`, asserting every frame is a `FromRadio` packet.
    fn decode_all_packets(codec: &mut MeshtasticCodec, src: &mut BytesMut) -> Vec<FromRadio> {
        let mut packets = Vec::new();
        while let Some(frame) = codec.decode(src).expect("decode should not error") {
            match frame {
                Frame::Packet(packet) => packets.push(packet),
                Frame::RadioLog(line) => panic!("unexpected RadioLog frame: {line:?}"),
            }
        }
        packets
    }

    /// Test for processing a single complete packet.
    /// The buffer contains one complete packet with a correct header, length, and matching data size.
    /// Expected behavior is that the function extracts this packet correctly and leaves the buffer empty if no extra bytes are present.
    #[test]
    fn process_single_complete_packet() {
        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::from(encoded_packet_1.data());

        let frame = codec.decode(&mut src).unwrap();
        assert_eq!(frame, Some(Frame::Packet(packet_1)));
        assert_eq!(codec.decode(&mut src).unwrap(), None);
        assert!(src.is_empty());
    }

    /// Test for handling an incomplete packet at the buffer's end.
    /// The buffer ends with a partial packet, only the header and part of the length are present.
    /// Expected behavior is that the function should not process this incomplete packet and leave it in the buffer for the next chunk to complete it.
    #[test]
    fn handle_incomplete_packet_at_end() {
        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let (_packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);

        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();

        let incomplete_encoded_packet_2: Vec<u8> =
            encoded_packet_2.data_vec().into_iter().take(6).collect();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        src.extend_from_slice(encoded_packet_1.data());
        src.extend_from_slice(&incomplete_encoded_packet_2);

        let packets = decode_all_packets(&mut codec, &mut src);
        assert_eq!(packets, vec![packet_1]);
        assert_eq!(src.len(), 6);
        assert_eq!(&src[..], &incomplete_encoded_packet_2[..]);
    }

    /// Test for processing multiple complete packets in a buffer.
    /// The buffer contains several complete packets back-to-back.
    /// Expected behavior is that the function processes and extracts all packets, leaving the buffer empty if no incomplete packet is trailing.
    #[test]
    fn process_multiple_complete_packets() {
        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let (packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);

        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        src.extend_from_slice(encoded_packet_1.data());
        src.extend_from_slice(encoded_packet_2.data());

        let packets = decode_all_packets(&mut codec, &mut src);
        assert_eq!(packets, vec![packet_1, packet_2]);
        assert!(src.is_empty());
    }

    /// Test for processing a buffer containing a valid packet followed by a malformed packet.
    /// The expected behavior is to process the first valid packet, then recognize the malformed packet and discard it,
    /// and resume processing with the next valid packet.
    #[test]
    fn handle_malformed_packet_amid_valid_packets() {
        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let payload_variant_3 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, 1);
        let (_packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, 2);
        let (packet_3, packet_data_3) = mock_encoded_from_radio_packet(payload_variant_3, 3);

        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();
        let encoded_packet_3 = format_data_packet(packet_data_3.into()).unwrap();

        let malformed_encoded_packet_2: Vec<u8> =
            encoded_packet_2.data_vec().into_iter().take(6).collect();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();

        src.extend_from_slice(encoded_packet_1.data());
        assert_eq!(decode_all_packets(&mut codec, &mut src), vec![packet_1]);

        src.extend_from_slice(&malformed_encoded_packet_2);
        assert!(decode_all_packets(&mut codec, &mut src).is_empty());

        src.extend_from_slice(encoded_packet_3.data());
        assert_eq!(decode_all_packets(&mut codec, &mut src), vec![packet_3]);
        assert!(src.is_empty());
    }

    /// Test for handling a buffer that ends with a false start (`0x94` without `0xc3` following).
    /// Expected behavior is that the function leaves the byte in the buffer, waiting for the next chunk to resolve the ambiguity.
    #[test]
    fn handle_buffer_ending_with_false_start() {
        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        src.extend_from_slice(encoded_packet_1.data());
        src.extend_from_slice(&[0x94]);

        let packets = decode_all_packets(&mut codec, &mut src);
        assert_eq!(packets, vec![packet_1]);
        assert_eq!(&src[..], &[0x94]);
    }

    /// Test for processing a packet when the buffer starts with `0x94` but the next byte is not `0xc3`,
    /// and no valid packet header (`0x94 0xc3`) appears later in the buffer.
    /// Expected behavior is that the function should clear the buffer until a valid packet header is found or the buffer is proven to contain no valid packets.
    #[test]
    fn clear_buffer_on_invalid_packet_start() {
        let malformed_packet = [0x94, 0x00, 0x94, 0x94, 0x00];
        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::from(&malformed_packet[..]);

        let packets = decode_all_packets(&mut codec, &mut src);
        assert!(packets.is_empty());
        assert!(src.is_empty());
    }

    /// Test for processing packets when the buffer contains multiple instances of `0x94` not followed by `0xc3`
    /// before finally presenting a valid packet header.
    /// Expected behavior is that the function discards all bytes up to the first valid packet header and then processes the valid packet(s) thereafter.
    #[test]
    fn process_after_repeated_false_starts() {
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();

        let malformed_packet_1 = [0x94, 0x00, 0x94, 0x94, 0x00];

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        src.extend_from_slice(&malformed_packet_1);
        assert!(decode_all_packets(&mut codec, &mut src).is_empty());

        src.extend_from_slice(encoded_packet_2.data());
        assert_eq!(decode_all_packets(&mut codec, &mut src), vec![packet_2]);
        assert!(src.is_empty());
    }

    /// Test for processing a large packet that spans multiple buffer chunks.
    /// Expected behavior is that the function correctly aggregates data across chunks until the full packet is received and then processes it.
    #[test]
    fn process_large_packet_spanning_multiple_chunks() {
        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();

        let chunk_1: Vec<u8> = encoded_packet_1.data_vec().into_iter().take(6).collect();
        let chunk_2: Vec<u8> = encoded_packet_1.data_vec().into_iter().skip(6).collect();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();

        src.extend_from_slice(&chunk_1);
        assert!(decode_all_packets(&mut codec, &mut src).is_empty());

        src.extend_from_slice(&chunk_2);
        assert_eq!(decode_all_packets(&mut codec, &mut src), vec![packet_1]);
        assert!(src.is_empty());
    }

    /// Test for processing a packet with length bytes indicating a length of 0.
    /// Expected behavior is that the function processes the packet as an empty packet and continues with the next packet in the buffer.
    /// Note: An "empty" packet is interpreted as a packet with an empty payload variant with a packet id of 0.
    #[test]
    fn process_packet_with_zero_length() {
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();

        let encoded_zero_length_packet = [0x94, 0xc3, 0x00, 0x00];

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        src.extend_from_slice(&encoded_zero_length_packet);
        src.extend_from_slice(encoded_packet_2.data());

        let empty_packet = protobufs::FromRadio {
            id: 0,
            payload_variant: None,
        };

        let packets = decode_all_packets(&mut codec, &mut src);
        assert_eq!(packets, vec![empty_packet, packet_2]);
        assert!(src.is_empty());
    }

    /// Test for detecting malformed packets when the packet data contains 0x94 bytes followed by 0x94 0xC3 sequence.
    /// The test ensures the algorithm can distinguish between valid use of 0x94 bytes in the data and the presence
    /// of a 0x94 0xC3 sequence that invalidates the packet. A packet starts correctly with 0x94 0xC3 and includes valid
    /// length bytes, but contains a 0x94 byte followed, at some point, by a 0x94 0xC3 sequence within the data payload.
    /// Expected behavior is for the algorithm to identify this packet as malformed upon detecting the 0x94 0xC3 sequence
    /// within the payload and to discard the malformed packet.
    #[test]
    fn detect_malformed_packets_with_internal_header_sequence() {
        let valid_packet = protobufs::FromRadio {
            id: 42,
            payload_variant: Some(protobufs::from_radio::PayloadVariant::MyInfo(
                protobufs::MyNodeInfo::default(),
            )),
        };
        let encoded_valid = format_data_packet(valid_packet.encode_to_vec().into()).unwrap();

        // Malformed packet declaring length 10, containing a standalone 0x94 followed by invalid
        // protobuf wire bytes (0xff) and then a valid packet starting with [0x94, 0xc3] inside its declared payload range.
        let mut src = BytesMut::new();
        src.extend_from_slice(&[0x94, 0xc3, 0x00, 10, 0xff, 0x94, 0xff]);
        src.extend_from_slice(encoded_valid.data());

        let mut codec = MeshtasticCodec::new();
        let packets = decode_all_packets(&mut codec, &mut src);
        assert_eq!(packets, vec![valid_packet]);
        assert!(src.is_empty());
    }

    /// `outer_packet`'s serialized bytes coincidentally contain [0x94, 0xc3],
    /// the same collision that caused a real hang. Identity fields are
    /// placeholders; device_metrics is unchanged
    /// since that's what produces the collision. Expected behavior is that both
    /// packets decode, since the coincidental match doesn't mean outer_packet
    /// is malformed.
    #[test]
    fn resync_after_coincidental_header_match_inside_packet_payload() {
        let outer_packet = protobufs::FromRadio {
            payload_variant: Some(protobufs::from_radio::PayloadVariant::NodeInfo(
                protobufs::NodeInfo {
                    device_metrics: Some(protobufs::DeviceMetrics {
                        uptime_seconds: Some(14655892),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )),
            ..Default::default()
        };
        let encoded_outer = format_data_packet(outer_packet.encode_to_vec().into()).unwrap();
        assert!(
            encoded_outer.data_vec()[HEADER_SIZE..]
                .windows(2)
                .any(|w| w == HEADER_MAGIC),
            "outer_packet no longer reproduces the coincidental header collision"
        );

        let recovery_packet = protobufs::FromRadio {
            id: 0,
            payload_variant: Some(protobufs::from_radio::PayloadVariant::NodeInfo(
                protobufs::NodeInfo {
                    num: 987654321,
                    ..Default::default()
                },
            )),
        };
        let encoded_recovery = format_data_packet(recovery_packet.encode_to_vec().into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        src.extend_from_slice(encoded_outer.data());
        src.extend_from_slice(encoded_recovery.data());

        let packets = decode_all_packets(&mut codec, &mut src);
        assert_eq!(packets, vec![outer_packet, recovery_packet]);
        assert!(src.is_empty());
    }

    /// Feed some complete log lines with a trailing newline
    #[test]
    fn process_log_lines() {
        let mut codec = MeshtasticCodec::new();
        let log_data = b"[DeviceTelemetry] Send packet to mesh\r\nSecond line of logs\n";
        let mut src = BytesMut::from(&log_data[..]);

        let mut frames = Vec::new();
        while let Some(frame) = codec.decode(&mut src).unwrap() {
            frames.push(frame);
        }

        assert_eq!(
            frames,
            vec![
                Frame::RadioLog("[DeviceTelemetry] Send packet to mesh".to_string()),
                Frame::RadioLog("Second line of logs".to_string()),
            ]
        );
        assert!(src.is_empty());
        assert!(codec.log_line.is_empty());
    }

    /// Feed a partial log line across multiple chunks (including split multi-byte UTF-8)
    #[test]
    fn process_partial_log_line() {
        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::from(&b"Hello, partial "[..]);

        assert_eq!(codec.decode(&mut src).unwrap(), None);
        assert!(src.is_empty());

        // Split a 3-byte UTF-8 character ('€' = [0xE2, 0x82, 0xAC]) across two chunks
        src.extend_from_slice(&[0xE2, 0x82]);
        assert_eq!(codec.decode(&mut src).unwrap(), None);
        assert!(src.is_empty());

        src.extend_from_slice(&[0xAC, b'\n']);
        assert_eq!(
            codec.decode(&mut src).unwrap(),
            Some(Frame::RadioLog("Hello, partial €".to_string()))
        );
        assert_eq!(codec.decode(&mut src).unwrap(), None);
        assert!(src.is_empty());
        assert!(codec.log_line.is_empty());
    }

    /// Interleave log line, a valid packet, and another log line
    #[test]
    fn process_interleaved_logs_and_packet() {
        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        src.extend_from_slice(b"[RadioIf] Can not send yet, busyRx\n");
        src.extend_from_slice(encoded_packet_1.data());
        src.extend_from_slice(b"Some device log after packet\n");

        let mut frames = Vec::new();
        while let Some(frame) = codec.decode(&mut src).unwrap() {
            frames.push(frame);
        }

        assert_eq!(
            frames,
            vec![
                Frame::RadioLog("[RadioIf] Can not send yet, busyRx".to_string()),
                Frame::Packet(packet_1),
                Frame::RadioLog("Some device log after packet".to_string()),
            ]
        );
        assert!(src.is_empty());
        assert!(codec.log_line.is_empty());
    }

    #[test]
    fn encode_to_radio_packet() {
        let to_radio = ToRadio {
            payload_variant: Some(protobufs::to_radio::PayloadVariant::Heartbeat(
                protobufs::Heartbeat::default(),
            )),
        };
        let expected = format_data_packet(to_radio.encode_to_vec().into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut dst = BytesMut::new();
        codec.encode(to_radio, &mut dst).unwrap();

        assert_eq!(&dst[..], expected.data());
    }

    #[test]
    fn encode_raw_to_radio_packet() {
        let raw_packet: EncodedToRadioPacket = vec![0x01, 0x02, 0x03, 0x04].into();
        let expected = format_data_packet(raw_packet.clone()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut dst = BytesMut::new();
        codec.encode(raw_packet, &mut dst).unwrap();

        assert_eq!(&dst[..], expected.data());
    }

    #[test]
    fn encode_rejects_oversized_packet() {
        let oversized_raw: EncodedToRadioPacket = vec![0u8; MAX_PACKET_SIZE + 1].into();
        let mut codec = MeshtasticCodec::new();
        let mut dst = BytesMut::new();

        let err = codec.encode(oversized_raw, &mut dst).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidaDataSize {
                data_length
            } if data_length == MAX_PACKET_SIZE + 1
        ));
        assert!(dst.is_empty());
    }

    #[test]
    fn chunked_packet_with_coincidental_header_match_inside_payload() {
        let outer_packet = protobufs::FromRadio {
            payload_variant: Some(protobufs::from_radio::PayloadVariant::NodeInfo(
                protobufs::NodeInfo {
                    device_metrics: Some(protobufs::DeviceMetrics {
                        uptime_seconds: Some(14655892),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )),
            ..Default::default()
        };
        let encoded_outer = format_data_packet(outer_packet.encode_to_vec().into()).unwrap();
        let outer_bytes = encoded_outer.data();
        let magic_pos = HEADER_SIZE
            + outer_bytes[HEADER_SIZE..]
                .windows(2)
                .position(|w| w == HEADER_MAGIC)
                .expect("outer_packet must contain coincidental magic bytes");

        let recovery_packet = protobufs::FromRadio {
            id: 7,
            payload_variant: Some(protobufs::from_radio::PayloadVariant::MyInfo(
                protobufs::MyNodeInfo::default(),
            )),
        };
        let encoded_recovery = format_data_packet(recovery_packet.encode_to_vec().into()).unwrap();

        // Split right after the coincidental [0x94, 0xc3] inside outer_packet's payload.
        let split_at = magic_pos + HEADER_MAGIC.len();
        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::from(&outer_bytes[..split_at]);

        assert!(decode_all_packets(&mut codec, &mut src).is_empty());
        assert_eq!(src.len(), split_at);

        src.extend_from_slice(&outer_bytes[split_at..]);
        src.extend_from_slice(encoded_recovery.data());

        assert_eq!(
            decode_all_packets(&mut codec, &mut src),
            vec![outer_packet, recovery_packet]
        );
        assert!(src.is_empty());
        assert!(codec.log_line.is_empty());
    }

    #[test]
    fn resync_after_false_header_match_with_implausible_size() {
        let payload_variant =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant, 123);
        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        // Standard false header with declared length > MAX_PACKET_SIZE.
        src.extend_from_slice(&[0x94, 0xc3, 0xfe, 0x06]);
        // Overlapping magic inside the length bytes of an oversized header, immediately followed by a valid packet.
        src.extend_from_slice(&[0x94, 0xc3, 0x94, 0xc3]);
        src.extend_from_slice(&encoded_packet_1.data()[2..]);

        assert_eq!(decode_all_packets(&mut codec, &mut src), vec![packet_1]);
        assert!(src.is_empty());
        assert!(codec.log_line.is_empty());
    }

    #[test]
    fn discard_undecodable_frame_without_log_pollution() {
        let payload_variant =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (valid_packet, packet_data) = mock_encoded_from_radio_packet(payload_variant, 99);
        let encoded_valid = format_data_packet(packet_data.into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        // Complete 4-byte payload containing invalid protobuf wire type and a newline (0x0a),
        // with no internal [0x94, 0xc3].
        src.extend_from_slice(&[0x94, 0xc3, 0x00, 0x04, 0xff, 0x0a, 0xff, 0xff]);
        src.extend_from_slice(encoded_valid.data());

        assert_eq!(decode_all_packets(&mut codec, &mut src), vec![valid_packet]);
        assert!(src.is_empty());
        assert!(codec.log_line.is_empty());
    }

    #[test]
    fn discard_malformed_frame_with_oversized_internal_magic_without_log_pollution() {
        let payload_variant =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let (valid_packet, packet_data) = mock_encoded_from_radio_packet(payload_variant, 100);
        let encoded_valid = format_data_packet(packet_data.into()).unwrap();

        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::new();
        // Malformed frame whose payload contains [0x94, 0xc3] with an oversized length (0xffff)
        // followed by ASCII + '\n'; rejecting the invalid internal header must discard the whole
        // frame rather than emitting "a" as a RadioLog.
        src.extend_from_slice(&[
            0x94, 0xc3, 0x00, 0x07, 0xff, 0x94, 0xc3, 0xff, 0xff, b'a', b'\n',
        ]);
        src.extend_from_slice(encoded_valid.data());

        assert_eq!(decode_all_packets(&mut codec, &mut src), vec![valid_packet]);
        assert!(src.is_empty());
        assert!(codec.log_line.is_empty());
    }

    #[test]
    fn accept_valid_packet_with_unknown_protobuf_field_containing_magic() {
        let mut codec = MeshtasticCodec::new();
        // FromRadio { id: 1 } plus unknown field 99 (tag 0x9a, 0x06, len 5) containing [0x94, 0xc3, 0x00, 0x00, 0x61],
        // split right after the embedded complete empty frame [0x94, 0xc3, 0x00, 0x00].
        let mut src = BytesMut::from(
            &[
                0x94, 0xc3, 0x00, 0x0a, 0x08, 0x01, 0x9a, 0x06, 0x05, 0x94, 0xc3, 0x00, 0x00,
            ][..],
        );

        assert!(decode_all_packets(&mut codec, &mut src).is_empty());

        src.extend_from_slice(&[0x61]);
        let expected = protobufs::FromRadio {
            id: 1,
            payload_variant: None,
        };
        assert_eq!(decode_all_packets(&mut codec, &mut src), vec![expected]);
        assert!(src.is_empty());
        assert!(codec.log_line.is_empty());
    }

    #[test]
    fn bound_log_line_length() {
        let mut codec = MeshtasticCodec::new();
        let mut src = BytesMut::from(&vec![b'a'; MAX_LOG_LINE_SIZE + 10][..]);

        let first = codec.decode(&mut src).unwrap();
        assert_eq!(first, Some(Frame::RadioLog("a".repeat(MAX_LOG_LINE_SIZE))));
        assert_eq!(codec.decode(&mut src).unwrap(), None);
        assert_eq!(codec.log_line.len(), 10);
        assert!(src.is_empty());
    }

    #[test]
    fn decode_eof_flushes_log_and_discards_incomplete_data() {
        let mut codec = MeshtasticCodec::new();

        // Trailing unterminated log line is flushed at EOF.
        let mut src = BytesMut::from(&b"final log line"[..]);
        assert_eq!(
            codec.decode_eof(&mut src).unwrap(),
            Some(Frame::RadioLog("final log line".to_string()))
        );
        assert_eq!(codec.decode_eof(&mut src).unwrap(), None);
        assert!(src.is_empty());

        // Trailing lone 0x94 is discarded cleanly at EOF without error.
        let mut src = BytesMut::from(&[0x94][..]);
        assert_eq!(codec.decode_eof(&mut src).unwrap(), None);
        assert!(src.is_empty());

        // Trailing incomplete frame is discarded cleanly at EOF without error.
        let mut src = BytesMut::from(&[0x94, 0xc3, 0x00, 0x10, 0x01, 0x02][..]);
        assert_eq!(codec.decode_eof(&mut src).unwrap(), None);
        assert!(src.is_empty());
    }
}
