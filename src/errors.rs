use thiserror::Error;

use crate::connections::stream_buffer::StreamBufferError;

#[derive(Error, Debug)]
pub enum Error {
    // ? Might want a more user-friendly error type for general
    // ? failure to send a packet (e.g., channel failure)
    // #[error("Failed to send packet")]
    // PacketSendFailure,
    #[error(transparent)]
    EncodeError(#[from] prost::EncodeError),
    #[error(transparent)]
    ChannelWriteFailure(#[from] tokio::sync::mpsc::error::SendError<Vec<u8>>),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Packet handler failed with error {source:?}")]
    PacketHandlerFailure {
        source: Box<dyn std::error::Error + 'static>,
    },
    #[error("Failed to read from stream with error {source:?}")]
    StreamReadError {
        source: Box<dyn std::error::Error + 'static>,
    },
    #[error("Failed to write to stream with error {source:?}")]
    StreamWriteError {
        source: Box<dyn std::error::Error + 'static>,
    },
    #[error(transparent)]
    StreamBufferError(#[from] StreamBufferError),
    #[error("Failed to build input data stream with error {source:?}")]
    StreamBuildError {
        source: Box<dyn std::error::Error + 'static>,
        description: String,
    },
}
