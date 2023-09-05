use thiserror::Error;

/// This enum defines the possible errors that can occur within the public API of the library.
#[derive(Error, Debug)]
pub enum Error {
    /// An error indicating that the library failed to encode a protocol buffer message.
    #[error(transparent)]
    EncodeError(#[from] prost::EncodeError),

    /// An error indicating that the library failed to write to an internal data channel.
    #[error(transparent)]
    ChannelWriteFailure(#[from] tokio::sync::mpsc::error::SendError<Vec<u8>>),

    /// An error indicating that the library failed to join a spawned worker task.
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    /// An error indicating that a struct implementing the `PacketRouter` trait failed to handle a packet.
    #[error("Packet handler failed with error {source:?}")]
    PacketHandlerFailure {
        source: Box<dyn std::error::Error + 'static>,
    },

    /// An error indicating that the library failed to read from the from_radio data stream half implementing `AsyncReadExt`.
    #[error("Failed to read from stream with error {source:?}")]
    StreamReadError {
        source: Box<dyn std::error::Error + 'static>,
    },

    /// An error indicating that the library failed to write to the to_radio data stream half implementing `AsyncWriteExt`.
    #[error("Failed to write to stream with error {source:?}")]
    StreamWriteError {
        source: Box<dyn std::error::Error + 'static>,
    },

    /// An error indicating that the library failed to build a data stream within a stream builder utility function.
    #[error("Failed to build input data stream with error {source:?}")]
    StreamBuildError {
        source: Box<dyn std::error::Error + 'static>,
        description: String,
    },
}
