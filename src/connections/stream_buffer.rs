use crate::protobufs;
use log::{debug, error, trace, warn};
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
    #[error("Could not find header byte 0x94 in buffer")]
    MissingHeaderByte,
    #[error("Incorrect framing byte: got {framing_byte}, expected 0xc3")]
    IncorrectFramingByte { framing_byte: u8 },
    #[error("Buffer data is shorter than packet header size: buffer contains {buffer_size} bytes, expected at least {packet_size} bytes")]
    IncompletePacket {
        buffer_size: usize,
        packet_size: usize,
    },
    #[error("Buffer does not contain a value at MSB buffer index of {msb_index}")]
    MissingMSB { msb_index: usize },
    #[error("Buffer does not contain a value at LSB buffer index of {lsb_index}")]
    MissingLSB { lsb_index: usize },
    #[error("Next packet index could not be found, received value of None")]
    MissingNextPacketIndex,
    #[error("Detected malformed packet, packet buffer contains a framing byte at index {framing_byte_index}")]
    MalformedPacket { framing_byte_index: usize },
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
                    StreamBufferError::MissingHeaderByte => break, // Wait for more data
                    StreamBufferError::IncorrectFramingByte { .. } => break, // Wait for more data
                    StreamBufferError::IncompletePacket { .. } => break, // Wait for more data
                    StreamBufferError::MissingMSB { .. } => break, // Wait for more data
                    StreamBufferError::MissingLSB { .. } => break, // Wait for more data
                    StreamBufferError::MissingNextPacketIndex => break, // Wait for more data
                    StreamBufferError::MalformedPacket { .. } => continue, // Don't need more data to continue, purge from buffer
                    StreamBufferError::DecodeFailure { .. } => continue, // Don't need more data to continue, ignore decode failure
                },
            };

            match self.decoded_packet_tx.send(decoded_packet) {
                Ok(_) => continue,
                Err(e) => {
                    error!("Failed to send decoded packet: {}", e.to_string());
                    break;
                }
            };
        }
    }

    /// An internal helper function that is called iteratively on the internal buffer. This
    /// function attempts to decode the buffer into a valid FromRadio packet. This function
    /// will return an error if the buffer does not contain enough data to decode a packet or
    /// if a packet is malformed. This function will return a packet if the buffer contains
    /// enough data to decode a packet, and is able to successfully decode the packet.
    ///
    /// **Note:** This function should only be called when not all received data in the buffer has been processed.
    fn process_packet_buffer(&mut self) -> Result<protobufs::FromRadio, StreamBufferError> {
        trace!("Packet buffer: {:?}", self.buffer);

        // Check that the buffer can potentially contain a packet header
        if self.buffer.len() < PACKET_HEADER_SIZE {
            debug!("Buffer data is shorter than packet header size, failing");
            return Err(StreamBufferError::IncompletePacket {
                buffer_size: self.buffer.len(),
                packet_size: PACKET_HEADER_SIZE,
            });
        }

        let mut framing_index = self.get_framing_index()?;

        // Drop beginning of buffer if the framing byte is found later in the buffer.
        // It is not possible to make a valid packet if the framing byte is not at the beginning.
        // We do this because the framing byte is the only way to determine the start of a packet,
        // and it needs to be updated after each removal of a malformed packet.
        // ! This needs to be done before the framing byte is accessed, as not doing so blocks
        // ! the processing of valid packets when an invalid packet is at the beginning of the buffer
        // ! For example, having 0xc3 as the first byte in the buffer followed by a valid packet will break
        while framing_index > 0 {
            debug!(
                "Found framing byte at index {}, shifting buffer",
                framing_index
            );

            self.buffer = self.buffer[framing_index..].to_vec();

            log::trace!("Buffer after shifting: {:?}", self.buffer);

            framing_index = self.get_framing_index()?;
        }

        // Note: the framing index should always be 0 at this point, keeping for clarity
        let incoming_packet_data_size = self.extract_data_size_from_header(framing_index)?;

        self.validate_packet_in_buffer(incoming_packet_data_size, framing_index)?;

        // Get packet data, excluding magic bytes
        let packet_data =
            self.extract_packet_from_buffer(incoming_packet_data_size, framing_index)?;

        // Attempt to decode the current packet
        let decoded_packet = protobufs::FromRadio::decode(packet_data.as_slice())?;

        Ok(decoded_packet)
    }

    // All valid packets start with the sequence [0x94 0xc3 size_msb size_lsb], where
    // size_msb and size_lsb collectively give the size of the incoming packet
    // Note that the maximum packet size currently stands at 240 bytes, meaning an MSB is not needed
    fn get_framing_index(&mut self) -> Result<usize, StreamBufferError> {
        match self.buffer.iter().position(|&b| b == 0x94) {
            Some(idx) => Ok(idx),
            None => {
                warn!("Could not find index of 0x94, purging buffer");
                self.buffer.clear(); // Clear buffer since no packets exist
                Err(StreamBufferError::MissingHeaderByte)
            }
        }
    }

    fn extract_data_size_from_header(
        &self,
        framing_index: usize,
    ) -> Result<usize, StreamBufferError> {
        // Get the "framing byte" after the start of the packet header, or fail if not found
        let framing_byte = match self.buffer.get(framing_index + 1) {
            Some(val) => val,
            None => {
                debug!("Could not find framing byte, waiting for more data");
                return Err(StreamBufferError::IncompletePacket {
                    buffer_size: self.buffer.len(),
                    packet_size: PACKET_HEADER_SIZE,
                });
            }
        };

        // Check that the framing byte is correct, and fail if not
        if *framing_byte != 0xc3 {
            warn!("Framing byte {} not equal to 0xc3", framing_byte);
            return Err(StreamBufferError::IncorrectFramingByte {
                framing_byte: *framing_byte,
            });
        }

        // Get the MSB of the packet header size, or wait to receive all data
        let msb_index: usize = 2;
        let msb = match self.buffer.get(msb_index) {
            Some(val) => val,
            None => {
                warn!("Could not find value for MSB");
                return Err(StreamBufferError::MissingMSB { msb_index });
            }
        };

        // Get the LSB of the packet header size, or wait to receive all data
        let lsb_index: usize = 3;
        let lsb = match self.buffer.get(lsb_index) {
            Some(val) => val,
            None => {
                warn!("Could not find value for LSB");
                return Err(StreamBufferError::MissingLSB { lsb_index });
            }
        };

        // Combine MSB and LSB of incoming packet size bytes
        // Recall that packet size doesn't include the first four magic bytes
        let incoming_packet_data_size: usize = usize::from(u16::from_le_bytes([*lsb, *msb]));

        return Ok(incoming_packet_data_size);
    }

    fn validate_packet_in_buffer(
        &mut self,
        packet_data_size: usize,
        framing_index: usize,
    ) -> Result<(), StreamBufferError> {
        if self.buffer.len() < PACKET_HEADER_SIZE + packet_data_size {
            return Err(StreamBufferError::IncompletePacket {
                buffer_size: self.buffer.len(),
                packet_size: packet_data_size,
            });
        }

        let packet_data_start_index = framing_index + PACKET_HEADER_SIZE;

        trace!(
            "Validating bytes in range [{}, {})",
            packet_data_start_index,
            packet_data_start_index + packet_data_size
        );

        // Packet is malformed if the start of another packet occurs within the defined limits of the current packet
        let malformed_packet_detector_index = self
            .buffer
            .iter()
            .enumerate()
            // Only want to check within the range of the current packet's data (not header)
            .filter(|&(i, _)| {
                packet_data_start_index <= i && i < packet_data_start_index + packet_data_size
            })
            .position(|(_, b)| *b == 0x94)
            // `position` returns the index from the filtered array, need to re-normalize to the original buffer
            .map(|idx| idx + packet_data_start_index);

        let malformed_packet_detector_byte = if let Some(index) = malformed_packet_detector_index {
            trace!("Found 0x94 at index {}", index);
            self.buffer.get(index + 1)
        } else {
            None
        };

        // If the byte after the 0x94 is 0xc3, this means not all bytes were received
        // in the current packet, meaning the packet is malformed and should be purged
        if *malformed_packet_detector_byte.unwrap_or(&0) == 0xc3 {
            error!("Detected malformed packet, purging");

            let next_packet_start_idx =
                malformed_packet_detector_index.ok_or(StreamBufferError::MissingNextPacketIndex)?;

            // Remove malformed packet from buffer
            self.buffer = self.buffer[next_packet_start_idx..].to_vec();

            return Err(StreamBufferError::MalformedPacket {
                framing_byte_index: next_packet_start_idx,
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
                packet_size: packet_data_size,
            });
        }

        let packet_size = PACKET_HEADER_SIZE + packet_data_size;

        // Extract packet with header before removing header
        let mut packet_data_with_header: Vec<u8> =
            self.buffer.drain(framing_index..packet_size).collect();

        trace!(
            "Extracted packet data with header of length {:?} from buffer: {:?}",
            packet_data_with_header.len(),
            packet_data_with_header
        );

        // Remove header bytes
        let packet_data: Vec<u8> = packet_data_with_header
            .drain(PACKET_HEADER_SIZE..)
            .collect();

        trace!(
            "Extracted packet data of length {:?} from buffer: {:?}",
            packet_data.len(),
            packet_data
        );

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
        id: u32,
        payload_variant: protobufs::from_radio::PayloadVariant,
    ) -> (protobufs::FromRadio, Vec<u8>) {
        let packet = protobufs::FromRadio {
            id,
            payload_variant: Some(payload_variant),
        };

        (packet.clone(), packet.encode_to_vec())
    }

    #[tokio::test]
    async fn decodes_valid_buffer_single_packet() {
        // Packet setup

        let payload_variant =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo {
                my_node_num: 1,
                ..Default::default()
            });

        let (packet, packet_data) = mock_encoded_from_radio_packet(1, payload_variant);
        let encoded_packet = format_data_packet(packet_data.into());

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Attempt to decode packet

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.process_incoming_bytes(encoded_packet.unwrap().data().into());

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet));
        assert_eq!(buffer.buffer.len(), 0);
    }

    #[tokio::test]
    async fn decodes_valid_buffer_two_packets() {
        // Packet setup

        let payload_variant1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo {
                my_node_num: 1,
                ..Default::default()
            });

        let payload_variant2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo {
                my_node_num: 2,
                ..Default::default()
            });

        let (packet1, packet_data1) = mock_encoded_from_radio_packet(1, payload_variant1);
        let (packet2, packet_data2) = mock_encoded_from_radio_packet(2, payload_variant2);

        let encoded_packet1 = format_data_packet(packet_data1.into()).unwrap();
        let encoded_packet2 = format_data_packet(packet_data2.into()).unwrap();

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        // Attempt to decode packets

        let mut buffer = StreamBuffer::new(mock_tx);
        buffer.buffer.append(&mut encoded_packet1.data_vec());
        buffer.process_incoming_bytes(encoded_packet2.data().into());

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet1));
        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(packet2));

        assert_eq!(buffer.buffer.len(), 0);
    }

    #[tokio::test]
    async fn should_purge_buffer_before_testing_framing_byte() {
        let payload_variant1 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo {
                my_node_num: 1,
                ..Default::default()
            });

        let payload_variant2 =
            protobufs::from_radio::PayloadVariant::MyInfo(protobufs::MyNodeInfo {
                my_node_num: 1,
                ..Default::default()
            });

        let (_packet1, packet_data1) = mock_encoded_from_radio_packet(1, payload_variant1);
        let (valid_packet, valid_packet_encoding) =
            mock_encoded_from_radio_packet(1, payload_variant2);

        let encoded_packet1 = format_data_packet(packet_data1.into()).unwrap();
        let encoded_packet2 = format_data_packet(valid_packet_encoding.into()).unwrap();

        // Remove first byte from encoded_packet1 to simulate a malformed packet
        let mut malformed_packet_encoding = encoded_packet1.data_vec();
        malformed_packet_encoding.remove(0);

        let (mock_tx, mut mock_rx) = unbounded_channel::<protobufs::FromRadio>();

        let mut buffer = StreamBuffer::new(mock_tx);

        // Add two malformed packets to the buffer to test `while` loop
        buffer.buffer.append(&mut malformed_packet_encoding.clone());
        buffer.buffer.append(&mut malformed_packet_encoding);

        buffer.process_incoming_bytes(encoded_packet2.data().into());

        assert_eq!(timeout_test(mock_rx.recv(), None).await, Some(valid_packet));
    }

    // #[tokio::test]
    // async fn should_handle_incomplete_header_at_start_of_buffer() {
    //   // TODO
    // }
}
