use thiserror::Error;

use crate::connections::wrappers::encoded_data::{
    EncodedToRadioPacket, EncodedToRadioPacketWithHeader, IncomingStreamData,
};

/// This enum defines the possible errors that can occur within the public API of the library.
#[derive(Error, Debug)]
pub enum Error {
    /// An error indicating that the user has entered a channel outside of the range of valid channels [0..7].
    #[error("Invalid channel {channel} entered. Valid channels are in the range [0..7]")]
    InvalidChannelIndex {
        /// The invalid channel index that was entered.
        channel: u32,
    },

    /// An error indicating that the library failed to encode a protocol buffer message.
    #[error(transparent)]
    EncodeError(#[from] prost::EncodeError),

    /// An error indicating that the library failed to join a spawned worker task.
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    /// An error indicating that a struct implementing the `PacketRouter` trait failed to handle a packet.
    #[error("Packet handler failed with error {source:?}")]
    PacketHandlerFailure {
        /// The source error that caused the packet handler failure.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// An error indicating that the library failed to build a data stream within a stream builder utility function.
    #[error("Failed to build input data stream with error {source:?}")]
    StreamBuildError {
        /// The source error that caused the stream build error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        /// The description of the stream build error.
        description: String,
    },

    /// An error indicating that too much data is being sent.
    /// NOTE: This error name is misspelled and should be treated as if it is `InvalidDataSize`.
    #[error("Trying to send too much data")]
    InvalidaDataSize {
        /// The length of the data of invalid size.
        data_length: usize,
    },

    /// An error indicating that the method failed to remove a packet header from a packet buffer
    /// due to the packet buffer being too small to contain a header.
    #[error("Failed to remove packet header from packet buffer due to insufficient data length: {packet}")]
    InsufficientPacketBufferLength {
        /// The packet that is too small to contain a header.
        packet: EncodedToRadioPacketWithHeader,
    },

    /// An error indicating that a function was passed a parameter that is not valid.
    #[error("Invalid function parameter: {source:?}")]
    InvalidParameter {
        /// The source error that caused the invalid parameter error.
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        /// The description of the invalid parameter.
        description: String,
    },

    /// An error indicating that the library failed when performing an operation on an internal data stream.
    #[error(transparent)]
    InternalStreamError(#[from] InternalStreamError),

    /// An error indicating that the library failed when performing an operation on an internal data channel.
    #[error(transparent)]
    InternalChannelError(#[from] InternalChannelError),
}

/// An enum that defines the possible internal errors that can occur within the library when handling streams.
#[warn(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum InternalStreamError {
    /// An error indicating that the library failed to read from the from_radio data stream half implementing `AsyncReadExt`.
    #[error("Failed to read from stream with error {source:?}")]
    StreamReadError {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// An error indicating that the library failed to write to the to_radio data stream half implementing `AsyncWriteExt`.
    #[error("Failed to write to stream with error {source:?}")]
    StreamWriteError {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// An error indicating the stream has reached its "end of file" and will likely no longer be able to produce bytes.
    #[error("Stream has reached EOF")]
    Eof,

    /// An error indicatiing that the connection has been lost and both reading and writing are
    /// not possible anymore.
    #[error("Connection lost")]
    ConnectionLost,
}

impl InternalStreamError {
    pub fn write_error(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::StreamWriteError {
            source: Box::new(err),
        }
    }
}

/// An enum that defines the possible internal errors that can occur within the library when handling data channels.
#[allow(clippy::enum_variant_names)]
#[derive(Error, Debug)]
pub enum InternalChannelError {
    /// An error indicating that the library failed to write to an internal data channel.
    #[error(transparent)]
    ToRadioWriteError(#[from] tokio::sync::mpsc::error::SendError<EncodedToRadioPacket>),

    /// An error indicating that the library failed to write to an internal data channel.
    #[error(transparent)]
    ToRadioWithHeaderWriteError(
        #[from] tokio::sync::mpsc::error::SendError<EncodedToRadioPacketWithHeader>,
    ),

    /// An error indicating that the library failed to write to an internal data channel.
    #[error(transparent)]
    IncomingStreamDataWriteError(#[from] tokio::sync::mpsc::error::SendError<IncomingStreamData>),

    /// An error indicating that a channel was closed prematurely.
    #[error("Channel unexpectedly closed")]
    ChannelClosedEarly,
}

#[derive(Error, Debug)]
#[error("Bluetooth low energy connection error")]
#[cfg(feature = "bluetooth-le")]
pub struct BleConnectionError();

mod test {
    #[allow(dead_code)]
    fn is_send<T: Send>() {}

    #[allow(dead_code)]
    fn is_sync<T: Sync>() {}

    #[test]
    fn test_send_sync() {
        is_send::<super::Error>();
        is_sync::<super::Error>();
    }
}
