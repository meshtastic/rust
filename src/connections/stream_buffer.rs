use crate::protobufs;
use log::{debug, error, info, warn};
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
        // Check that the buffer can potentially contain a packet header
        if self.buffer.len() < 4 {
            debug!("Buffer data is shorter than packet header size, failing");
            return Err(StreamBufferError::IncompletePacket {
                buffer_size: self.buffer.len(),
                packet_size: 4,
            });
        }

        // All valid packets start with the sequence [0x94 0xc3 size_msb size_lsb], where
        // size_msb and size_lsb collectively give the size of the incoming packet
        // Note that the maximum packet size currently stands at 240 bytes, meaning an MSB is not needed
        let framing_index = match self.buffer.iter().position(|&b| b == 0x94) {
            Some(idx) => idx,
            None => {
                warn!("Could not find index of 0x94, purging buffer");
                self.buffer.clear(); // Clear buffer since no packets exist
                return Err(StreamBufferError::MissingHeaderByte);
            }
        };

        // Get the "framing byte" after the start of the packet header, or fail if not found
        let framing_byte = match self.buffer.get(framing_index + 1) {
            Some(val) => val,
            None => {
                debug!("Could not find framing byte, waiting for more data");
                return Err(StreamBufferError::IncompletePacket {
                    buffer_size: self.buffer.len(),
                    packet_size: 4,
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

        // Drop beginning of buffer if the framing byte is found later in the buffer
        // It is not possible to make a valid packet if the framing byte is not at the beginning
        if framing_index > 0 {
            debug!(
                "Found framing byte at index {}, shifting buffer",
                framing_index
            );

            self.buffer = self.buffer[framing_index..].to_vec();
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
        let incoming_packet_size: usize = usize::from(4 + u16::from_le_bytes([*lsb, *msb]));

        // Defer decoding until the correct number of bytes are received
        if self.buffer.len() < incoming_packet_size {
            warn!("Stream buffer size is less than size of packet");
            return Err(StreamBufferError::IncompletePacket {
                buffer_size: self.buffer.len(),
                packet_size: incoming_packet_size,
            });
        }

        // Get packet data, excluding magic bytes
        let packet: Vec<u8> = self.buffer[4..incoming_packet_size].to_vec();

        // Packet is malformed if the start of another packet occurs within the
        // defined limits of the current packet
        let malformed_packet_detector_index = packet.iter().position(|&b| b == 0x94);

        let malformed_packet_detector_byte = if let Some(index) = malformed_packet_detector_index {
            packet.get(index + 1)
        } else {
            None
        };

        // If the byte after the 0x94 is 0xc3, this means not all bytes were received
        // in the current packet, meaning the packet is malformed and should be purged
        if *malformed_packet_detector_byte.unwrap_or(&0) == 0xc3 {
            info!("Detected malformed packet, purging");

            let next_packet_start_idx =
                malformed_packet_detector_index.ok_or(StreamBufferError::MissingNextPacketIndex)?;

            // Remove malformed packet from buffer
            self.buffer = self.buffer[next_packet_start_idx..].to_vec();

            return Err(StreamBufferError::MalformedPacket {
                framing_byte_index: next_packet_start_idx,
            });
        }

        // Remove current packet from buffer based on start location of next packet
        self.buffer = self.buffer[incoming_packet_size..].to_vec();

        // Attempt to decode the current packet
        let decoded_packet = protobufs::FromRadio::decode(packet.as_slice())?;

        Ok(decoded_packet)
    }
}

#[cfg(test)]
mod tests {
    use crate::{protobufs, utils_internal::format_data_packet};
    use prost::Message;
    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

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

        assert_eq!(mock_rx.recv().await.unwrap(), packet);
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

        assert_eq!(mock_rx.recv().await.unwrap(), packet1);
        assert_eq!(mock_rx.recv().await.unwrap(), packet2);
        assert_eq!(buffer.buffer.len(), 0);
    }
}
