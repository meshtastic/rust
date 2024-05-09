use crate::protobufs;
use log::{debug, error, trace};
use prost::Message;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use super::wrappers::encoded_data::IncomingStreamData;

/// A struct that represents a buffer of bytes received from a radio stream.
/// This struct is used to store bytes received from a radio stream, and is
/// used to incrementally decode bytes from the received stream into valid
/// FromRadio packets.
#[derive(Clone, Debug)]
pub struct StreamBuffer {
    buffer: Vec<u8>,
    decoded_packet_tx: UnboundedSender<protobufs::FromRadio>,
}

/// An enum that represents the possible errors that can occur when processing
/// a stream buffer. These errors are used to determine whether the application
/// should wait to receive more data or if the buffer should be purged.
#[derive(Error, Debug, Clone)]
pub enum StreamBufferError {
    #[error("Could not find header sequence [0x94, 0xc3] in buffer")]
    MissingHeaderBytes,
    #[error("Incorrect framing byte: got {found_framing_byte}, expected 0xc3")]
    IncorrectFramingByte { found_framing_byte: u8 },
    #[error("Buffer data is shorter than packet header size: buffer contains {buffer_size} bytes, expected at least {packet_size} bytes")]
    IncompletePacket {
        buffer_size: usize,
        packet_size: usize,
    },
    #[error("Buffer does not contain a value at MSB buffer index of {msb_index}")]
    MissingMSB { msb_index: usize },
    #[error("Buffer does not contain a value at LSB buffer index of {lsb_index}")]
    MissingLSB { lsb_index: usize },
    #[error("Detected malformed packet, packet buffer contains a framing byte at index {next_packet_start_idx}")]
    MalformedPacket { next_packet_start_idx: usize },
    #[error(transparent)]
    DecodeFailure(#[from] prost::DecodeError),
}

const PACKET_HEADER_SIZE: usize = 4;

impl StreamBuffer {
    /// Creates a new StreamBuffer instance that will send decoded FromRadio packets
    /// to the given broadcast channel.
    pub fn new(decoded_packet_tx: UnboundedSender<protobufs::FromRadio>) -> Self {
        StreamBuffer {
            buffer: vec![],
            decoded_packet_tx,
        }
    }

    /// Takes in a portion of a stream message, stores it in a buffer,
    /// and attempts to decode the buffer into valid FromRadio packets.
    ///
    /// # Arguments
    ///
    /// * `message` - A vector of bytes received from a radio stream
    ///
    /// # Example
    ///
    /// ```
    /// let (rx, mut tx) = broadcast::channel::<protobufs::FromRadio>(32);
    /// let buffer = StreamBuffer::new(tx);
    ///
    /// while let Some(message) = stream.try_next().await? {
    ///    buffer.process_incoming_bytes(message);
    /// }
    /// ```
    pub fn process_incoming_bytes(&mut self, message: IncomingStreamData) {
        let message = message.data();
        self.buffer.append(Vec::from(message).as_mut());

        // While there are still bytes in the buffer and processing isn't completed,
        // continue processing the buffer
        while !self.buffer.is_empty() {
            let decoded_packet = match self.process_packet_buffer() {
                Ok(packet) => packet,
                Err(err) => match err {
                    StreamBufferError::MissingHeaderBytes => {
                        error!("Could not find header sequence [0x94, 0xc3], purging buffer and waiting for more data");

                        break; // Wait for more data
                    }
                    StreamBufferError::IncorrectFramingByte { found_framing_byte } => {
                        error!(
                            "Byte {} not equal to 0xc3, waiting for more data",
                            found_framing_byte
                        );

                        break; // Wait for more data
                    }
                    StreamBufferError::IncompletePacket {
                        buffer_size,
                        packet_size,
                    } => {
                        error!(
                            "Incomplete packet data, expected {} bytes, found {} bytes",
                            packet_size, buffer_size
                        );

                        break; // Wait for more data
                    }
                    StreamBufferError::MissingMSB { msb_index } => {
                        error!(
                            "Could not find MSB at index {}, waiting for more data",
                            msb_index
                        );

                        break; // Wait for more data
                    }
                    StreamBufferError::MissingLSB { lsb_index } => {
                        error!(
                            "Could not find LSB at index {}, waiting for more data",
                            lsb_index
                        );

                        break; // Wait for more data
                    }
                    StreamBufferError::MalformedPacket {
                        next_packet_start_idx,
                    } => {
                        error!(
                              "Detected malformed packet with next packet starting at index {}, purged malformed packet",
                              next_packet_start_idx
                          );

                        continue; // Don't need more data to continue, purge from buffer
                    }
                    StreamBufferError::DecodeFailure { .. } => {
                        error!("Failed to decode chunk from packet, this does not affect the next iteration");

                        continue; // Don't need more data to continue, ignore decode failure
                    }
                },
            };

            trace!("Successfully decoded packet");

            match self.decoded_packet_tx.send(decoded_packet) {
                Ok(_) => {
                    trace!("Successfully sent decoded packet");
                    continue;
                }
                Err(e) => {
                    error!("Failed to send decoded packet: {}", e.to_string());
                    break;
                }
            };
        }

        trace!(
            "Processing complete, buffer contains {} bytes",
            self.buffer.len()
        );
    }

    /// An internal helper function that is called iteratively on the internal buffer. This
    /// function attempts to decode the buffer into a valid FromRadio packet. This function
    /// will return an error if the buffer does not contain enough data to decode a packet or
    /// if a packet is malformed. This function will return a packet if the buffer contains
    /// enough data to decode a packet, and is able to successfully decode the packet.
    ///
    /// **Note:** This function should only be called when not all received data in the buffer has been processed.
    fn process_packet_buffer(&mut self) -> Result<protobufs::FromRadio, StreamBufferError> {
        trace!(
            "Packet buffer with length {:?}: {:?}",
            self.buffer.len(),
            self.buffer
        );

        // Check that the buffer can potentially contain a packet header
        if self.buffer.len() < PACKET_HEADER_SIZE {
            return Err(StreamBufferError::IncompletePacket {
                buffer_size: self.buffer.len(),
                packet_size: PACKET_HEADER_SIZE,
            });
        }

        let framing_index = StreamBuffer::shift_buffer_to_first_valid_header(&mut self.buffer)?;

        // Note: the framing index should always be 0 at this point, keeping for clarity
        let incoming_packet_data_size = self.get_data_size_from_header(framing_index)?;

        self.validate_packet_in_buffer(incoming_packet_data_size, framing_index)?;

        // Get packet data, excluding magic bytes
        let packet_data =
            self.extract_packet_from_buffer(incoming_packet_data_size, framing_index)?;

        // Attempt to decode the current packet
        let decoded_packet = protobufs::FromRadio::decode(packet_data.as_slice())?;

        Ok(decoded_packet)
    }

    fn shift_buffer_to_first_valid_header(
        buffer: &mut Vec<u8>,
    ) -> Result<usize, StreamBufferError> {
        let mut framing_index = StreamBuffer::find_framing_index_or_clear_buffer(buffer)?;

        if framing_index != 0 {
            debug!(
                "Found framing byte at index {}, shifting buffer",
                framing_index
            );

            buffer.drain(0..framing_index);

            log::trace!("Buffer after shifting: {:?}", buffer);

            framing_index = StreamBuffer::find_framing_index_or_clear_buffer(buffer)?;
        }

        trace!("Returning framing index: {}", framing_index);

        Ok(framing_index)
    }

    fn find_framing_index_or_clear_buffer(
        buffer: &mut Vec<u8>,
    ) -> Result<usize, StreamBufferError> {
        let framing_index = match StreamBuffer::find_framing_index(buffer)? {
            Some(idx) => idx,
            None => {
                buffer.clear(); // Clear buffer since no packets exist
                return Err(StreamBufferError::MissingHeaderBytes);
            }
        };

        Ok(framing_index)
    }

    // All valid packets start with the sequence [0x94 0xc3 size_msb size_lsb], where
    // size_msb and size_lsb collectively give the size of the incoming packet
    // We need to also validate that, if the 0x94 is found and not at the end of the
    // buffer, that the next byte is 0xc3
    // Note that the maximum packet size currently stands at 240 bytes, meaning an MSB is not needed
    fn find_framing_index(buffer: &mut [u8]) -> Result<Option<usize>, StreamBufferError> {
        // Not possible to have a two-byte sequence in a buffer with less than two bytes
        // Vec::windows will also panic if the buffer is empty
        if buffer.len() < 2 {
            return Ok(None);
        }

        let framing_index = buffer.windows(2).position(|b| b == [0x94, 0xc3]);

        Ok(framing_index)
    }

    fn get_data_size_from_header(
        &mut self,
        framing_index: usize,
    ) -> Result<usize, StreamBufferError> {
        // Get the "framing byte" after the start of the packet header, or fail if not found
        let found_framing_byte = match self.buffer.get(framing_index + 1) {
            Some(val) => val.to_owned(),
            None => {
                debug!("Could not find framing byte, waiting for more data");
                return Err(StreamBufferError::IncompletePacket {
                    buffer_size: self.buffer.len(),
                    packet_size: PACKET_HEADER_SIZE,
                });
            }
        };

        // Check that the framing byte is correct, and fail if not
        if found_framing_byte != 0xc3 {
            return Err(StreamBufferError::IncorrectFramingByte { found_framing_byte });
        }

        // Get the MSB of the packet header size, or wait to receive all data
        let msb_index: usize = 2;
        let msb = match self.buffer.get(msb_index) {
            Some(val) => val,
            None => {
                return Err(StreamBufferError::MissingMSB { msb_index });
            }
        };

        // Get the LSB of the packet header size, or wait to receive all data
        let lsb_index: usize = 3;
        let lsb = match self.buffer.get(lsb_index) {
            Some(val) => val,
            None => {
                return Err(StreamBufferError::MissingLSB { lsb_index });
            }
        };

        // Combine MSB and LSB of incoming packet size bytes
        // Recall that packet size doesn't include the first four magic bytes
        let incoming_packet_data_size: usize = usize::from(u16::from_le_bytes([*lsb, *msb]));

        Ok(incoming_packet_data_size)
    }

    fn validate_packet_in_buffer(
        &mut self,
        packet_data_size: usize,
        framing_index: usize,
    ) -> Result<(), StreamBufferError> {
        if self.buffer.len() < PACKET_HEADER_SIZE + packet_data_size {
            return Err(StreamBufferError::IncompletePacket {
                buffer_size: self.buffer.len(),
                packet_size: PACKET_HEADER_SIZE + packet_data_size,
            });
        }

        let packet_data_start_index = framing_index + PACKET_HEADER_SIZE;
        let mut packet_data_end_index = packet_data_start_index + packet_data_size;

        // In the event that the last byte is 0x94, we need to account for the possibility of
        // the next byte being 0xc3, which would indicate that the packet is malformed.
        // We can only do this when the buffer has enough data to avoid a slice index panic.
        if self.buffer.len() > packet_data_end_index {
            packet_data_end_index += 1;
        }

        // trace!(
        //     "Validating bytes in range [{}, {})",
        //     packet_data_start_index,
        //     packet_data_start_index + packet_data_size
        // );

        let mut packet_buffer =
            self.buffer[packet_data_start_index..packet_data_end_index].to_vec();

        let next_packet_start_index = StreamBuffer::find_framing_index(&mut packet_buffer)?
            // We need to re-normalize to the original buffer since we're working with a sub-slice
            .map(|idx| idx + packet_data_start_index);

        if let Some(next_packet_start_idx) = next_packet_start_index {
            // Remove malformed packet from buffer
            self.buffer.drain(..next_packet_start_idx);

            return Err(StreamBufferError::MalformedPacket {
                next_packet_start_idx,
            });
        }

        Ok(())
    }

    fn extract_packet_from_buffer(
        &mut self,
        packet_data_size: usize,
        framing_index: usize,
    ) -> Result<Vec<u8>, StreamBufferError> {
        if self.buffer.len() < packet_data_size {
            return Err(StreamBufferError::IncompletePacket {
                buffer_size: self.buffer.len(),
                packet_size: PACKET_HEADER_SIZE + packet_data_size,
            });
        }

        let packet_start_index = framing_index;
        let packet_end_index = framing_index + PACKET_HEADER_SIZE + packet_data_size;

        // Extract packet with header before removing header
        let mut packet_data_with_header: Vec<u8> = self
            .buffer
            .drain(packet_start_index..packet_end_index)
            .collect();

        // trace!(
        //     "Extracted packet data with header of length {:?} from buffer: {:?}",
        //     packet_data_with_header.len(),
        //     packet_data_with_header
        // );

        // Remove header bytes
        let packet_data: Vec<u8> = packet_data_with_header
            .drain(PACKET_HEADER_SIZE..)
            .collect();

        // trace!(
        //     "Extracted packet data of length {:?} from buffer: {:?}",
        //     packet_data.len(),
        //     packet_data
        // );

        Ok(packet_data)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{protobufs, utils_internal::format_data_packet};
    use futures_util::FutureExt;
    use prost::Message;
    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

    async fn timeout_test<F, T>(future: F, timeout: impl Into<Option<Duration>>) -> T
    where
        F: FutureExt<Output = T> + Send,
    {
        let timeout_opt: Option<Duration> = timeout.into();
        let timeout_duration = timeout_opt.unwrap_or(Duration::from_millis(100));

        tokio::time::timeout(timeout_duration, future)
            .await
            .expect("Future timed out")
    }

    fn mock_encoded_from_radio_packet(
        payload_variant: protobufs::from_radio::PayloadVariant,
        id: impl Into<Option<u32>>,
    ) -> (protobufs::FromRadio, Vec<u8>) {
        let packet_id = id.into().unwrap_or(rand::random());

        let packet = protobufs::FromRadio {
            id: packet_id,
            payload_variant: Some(payload_variant),
        };

        (packet.clone(), packet.encode_to_vec())
    }

    /// Test for processing a single complete packet.
    /// The buffer contains one complete packet with a correct header, length, and matching data size.
    /// Expected behavior is that the function extracts this packet correctly and leaves the buffer empty if no extra bytes are present.
    #[tokio::test]
    async fn process_single_complete_packet() {
        // Arrange

        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(encoded_packet_1.data().into());

        // Assert

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_1));
        assert_eq!(buffer.buffer.len(), 0);
    }

    /// Test for handling an incomplete packet at the buffer's end.
    /// The buffer ends with a partial packet, only the header and part of the length are present.
    /// Expected behavior is that the function should not process this incomplete packet and leave it in the buffer for the next chunk to complete it.
    #[tokio::test]
    async fn handle_incomplete_packet_at_end() {
        // Arrange

        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let (_packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);

        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();

        let incomplete_encoded_packet_2 = encoded_packet_2
            .data_vec()
            .into_iter()
            .take(6)
            .collect::<Vec<u8>>();

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(encoded_packet_1.data().into());
        buffer.process_incoming_bytes(incomplete_encoded_packet_2.clone().into());

        // Assert

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_1));
        assert_eq!(buffer.buffer.len(), 6);
        assert_eq!(buffer.buffer, incomplete_encoded_packet_2);
    }

    /// Test for processing multiple complete packets in a buffer.
    /// The buffer contains several complete packets back-to-back.
    /// Expected behavior is that the function processes and extracts all packets, leaving the buffer empty if no incomplete packet is trailing.
    #[tokio::test]
    async fn process_multiple_complete_packets() {
        // Arrange

        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let (packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);

        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(encoded_packet_1.data().into());
        buffer.process_incoming_bytes(encoded_packet_2.data().into());

        // Assert

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_1));
        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_2));
        assert_eq!(buffer.buffer.len(), 0);
    }

    /// Test for processing a buffer containing a valid packet followed by a malformed packet.
    /// The expected behavior is to process the first valid packet, then recognize the malformed packet and discard it,
    /// and resume processing with the next valid packet.
    #[tokio::test]
    async fn handle_malformed_packet_amid_valid_packets() {
        // Arrange

        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());
        let payload_variant_3 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let (_packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);
        let (packet_3, packet_data_3) = mock_encoded_from_radio_packet(payload_variant_3, None);

        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();
        let encoded_packet_3 = format_data_packet(packet_data_3.into()).unwrap();

        let malformed_encoded_packet_2 = encoded_packet_2
            .data_vec()
            .into_iter()
            .take(6)
            .collect::<Vec<u8>>();

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(encoded_packet_1.data().into());
        buffer.process_incoming_bytes(malformed_encoded_packet_2.clone().into());
        buffer.process_incoming_bytes(encoded_packet_3.data().into());

        // Assert

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_1));
        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_3));
        assert_eq!(buffer.buffer.len(), 0);
    }

    /// Test for handling a buffer that ends with a false start (`0x94` without `0xc3` following).
    /// Expected behavior is that the function leaves the byte in the buffer, waiting for the next chunk to resolve the ambiguity.
    #[tokio::test]
    async fn handle_buffer_ending_with_false_start() {
        // Arrange

        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(encoded_packet_1.data().into());
        buffer.process_incoming_bytes(vec![0x94].into());

        // Assert

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_1));
        assert_eq!(buffer.buffer, vec![0x94]);
    }

    /// Test for processing a packet when the buffer starts with `0x94` but the next byte is not `0xc3`,
    /// and no valid packet header (`0x94 0xc3`) appears later in the buffer.
    /// Expected behavior is that the function should clear the buffer until a valid packet header is found or the buffer is proven to contain no valid packets.
    #[tokio::test]
    async fn clear_buffer_on_invalid_packet_start() {
        // Arrange

        let malformed_packet_1 = vec![0x94, 0x00, 0x94, 0x94, 0x00];

        let (mock_tx, mut _mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(malformed_packet_1.into());

        // Assert

        assert_eq!(buffer.buffer.len(), 0);
    }

    /// Test for processing packets when the buffer contains multiple instances of `0x94` not followed by `0xc3`
    /// before finally presenting a valid packet header.
    /// Expected behavior is that the function discards all bytes up to the first valid packet header and then processes the valid packet(s) thereafter.
    #[tokio::test]
    async fn process_after_repeated_false_starts() {
        // Arrange

        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();

        let malformed_packet_1 = vec![0x94, 0x00, 0x94, 0x94, 0x00];

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(malformed_packet_1.into());
        buffer.process_incoming_bytes(encoded_packet_2.data().into());

        // Assert

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_2));
        assert_eq!(buffer.buffer.len(), 0);
    }

    /// Test for processing a large packet that spans multiple buffer chunks.
    /// Expected behavior is that the function correctly aggregates data across chunks until the full packet is received and then processes it.
    #[tokio::test]
    async fn process_large_packet_spanning_multiple_chunks() {
        // Arrange

        let payload_variant_1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_1, packet_data_1) = mock_encoded_from_radio_packet(payload_variant_1, None);
        let encoded_packet_1 = format_data_packet(packet_data_1.into()).unwrap();

        let encoded_packet_1_chunk_1 = encoded_packet_1
            .clone()
            .data_vec()
            .into_iter()
            .take(6)
            .collect::<Vec<u8>>();

        let encoded_packet_1_chunk_2 = encoded_packet_1
            .data_vec()
            .into_iter()
            .skip(6)
            .collect::<Vec<u8>>();

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(encoded_packet_1_chunk_1.into());
        buffer.process_incoming_bytes(encoded_packet_1_chunk_2.into());

        // Assert

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_1));
        assert_eq!(buffer.buffer.len(), 0);
    }

    // /// Test for handling overlapping packet header and length bytes.
    // /// A valid packet header is immediately followed by bytes that could be interpreted as another packet header within the data segment.
    // /// Expected behavior is that the function processes the entire packet without misinterpreting internal data as a new header.
    // /// Note that this is an edge case, as current Meshtastic firmware does not allow for packets long enough for this to happen.
    // #[tokio::test]
    // async fn handle_overlapping_header_and_length_bytes() {}

    /// Test for processing a packet with length bytes indicating a length of 0.
    /// Expected behavior is that the function processes the packet as an empty packet and continues with the next packet in the buffer.
    /// Note: An "empty" packet is interpreted as a packet with an empty payload variant with a packet id of 0.
    #[tokio::test]
    async fn process_packet_with_zero_length() {
        let payload_variant_2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo::default());

        let (packet_2, packet_data_2) = mock_encoded_from_radio_packet(payload_variant_2, None);
        let encoded_packet_2 = format_data_packet(packet_data_2.into()).unwrap();

        let encoded_zero_length_packet = vec![0x94, 0xc3, 0x00, 0x00];

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Act

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(encoded_zero_length_packet.into());
        buffer.process_incoming_bytes(encoded_packet_2.data().into());

        // Assert

        let empty_packet = protobufs::FromRadio {
            id: 0,
            payload_variant: None,
        };

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(empty_packet));
        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet_2));
        assert_eq!(buffer.buffer.len(), 0);
    }

    /// Test for detecting malformed packets when the packet data contains 0x94 bytes followed by 0x94 0xC3 sequence.
    /// The test ensures the algorithm can distinguish between valid use of 0x94 bytes in the data and the presence
    /// of a 0x94 0xC3 sequence that invalidates the packet. A packet starts correctly with 0x94 0xC3 and includes valid
    /// length bytes, but contains a 0x94 byte followed, at some point, by a 0x94 0xC3 sequence within the data payload.
    /// Expected behavior is for the algorithm to identify this packet as malformed upon detecting the 0x94 0xC3 sequence
    /// within the payload and to discard the malformed packet.
    #[tokio::test]
    async fn detect_malformed_packets_with_internal_header_sequence() {}

    // TODO need to test that we update the framing index after shifting the buffer
}
