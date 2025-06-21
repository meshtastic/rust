use crate::errors_internal::{Error, InternalChannelError, InternalStreamError};
use crate::protobufs;
use crate::types::EncodedToRadioPacketWithHeader;
use crate::utils::format_data_packet;
use log::{debug, error, trace, warn};
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::spawn;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::connections::stream_buffer::StreamBuffer;

use super::wrappers::encoded_data::IncomingStreamData;

/// Interval for sending heartbeat packets to the radio (in seconds).
/// Needs to be less than this: https://github.com/meshtastic/firmware/blob/eb372c190ec82366998c867acc609a418130d842/src/SerialConsole.cpp#L8
pub const CLIENT_HEARTBEAT_INTERVAL: u64 = 5 * 60; // 5 minutes

pub fn spawn_read_handler<R>(
    cancellation_token: CancellationToken,
    read_stream: R,
    read_output_tx: UnboundedSender<IncomingStreamData>,
) -> JoinHandle<Result<(), Error>>
where
    R: AsyncReadExt + Send + Unpin + 'static,
{
    let handle = start_read_handler(read_stream, read_output_tx.clone());

    spawn(async move {
        // Check for cancellation signal or handle termination
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                debug!("Read handler cancelled");
                Ok(())
            }
            e = handle => {
                error!("Read handler unexpectedly terminated: {:#?}", e);
                e
            }
        }
    })
}

async fn start_read_handler<R>(
    read_stream: R,
    read_output_tx: UnboundedSender<IncomingStreamData>,
) -> Result<(), Error>
where
    R: AsyncReadExt + Send + Unpin + 'static,
{
    debug!("Started read handler");

    let mut read_stream = read_stream;

    loop {
        let mut buffer = [0u8; 1024];
        match read_stream.read(&mut buffer).await {
            Ok(0) => {
                warn!("read_stream has reached EOF");
                return Err(Error::InternalStreamError(InternalStreamError::Eof));
            }
            Ok(n) => {
                trace!("Read {} bytes from stream", n);
                let data: IncomingStreamData = buffer[..n].to_vec().into();
                trace!("Read data: {:?}", data);

                if let Err(e) = read_output_tx.send(data) {
                    error!("Failed to send data through channel");
                    return Err(Error::InternalChannelError(e.into()));
                }
            }

            // TODO check if port has fatally errored, and if so, tell UI
            Err(e) => {
                error!("Error reading from stream: {:?}", e);
                return Err(Error::InternalStreamError(
                    InternalStreamError::StreamReadError {
                        source: Box::new(e),
                    },
                ));
            }
        }
    }

    // trace!("Read handler finished");

    // Return type should be never (!)
}

pub fn spawn_write_handler<W>(
    cancellation_token: CancellationToken,
    write_stream: W,
    write_input_rx: tokio::sync::mpsc::UnboundedReceiver<EncodedToRadioPacketWithHeader>,
) -> JoinHandle<Result<(), Error>>
where
    W: AsyncWriteExt + Send + Unpin + 'static,
{
    let handle = start_write_handler(cancellation_token.clone(), write_stream, write_input_rx);

    spawn(async move {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                debug!("Write handler cancelled");
                Ok(())
            }
            write_result = handle => {
                if let Err(e) = &write_result {
                    error!("Write handler unexpectedly terminated {e:?}");
                }
                write_result
            }
        }
    })
}

async fn start_write_handler<W>(
    _cancellation_token: CancellationToken,
    mut write_stream: W,
    mut write_input_rx: tokio::sync::mpsc::UnboundedReceiver<EncodedToRadioPacketWithHeader>,
) -> Result<(), Error>
where
    W: AsyncWriteExt + Send + Unpin + 'static,
{
    debug!("Started write handler");

    while let Some(message) = write_input_rx.recv().await {
        trace!("Writing packet data: {:?}", message);

        if let Err(e) = write_stream.write(message.data()).await {
            error!("Error writing to stream: {:?}", e);
            return Err(Error::InternalStreamError(
                InternalStreamError::StreamWriteError {
                    source: Box::new(e),
                },
            ));
        }
    }

    debug!("Write handler finished");

    Ok(())
}

pub fn spawn_processing_handler(
    cancellation_token: CancellationToken,
    read_output_rx: UnboundedReceiver<IncomingStreamData>,
    decoded_packet_tx: UnboundedSender<protobufs::FromRadio>,
) -> JoinHandle<Result<(), Error>> {
    let handle = start_processing_handler(read_output_rx, decoded_packet_tx);

    spawn(async move {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                debug!("Message processing handler cancelled");
                Ok(())
            }
            _ = handle => {
                error!("Message processing handler unexpectedly terminated");
                Err(Error::InternalChannelError(InternalChannelError::ChannelClosedEarly {}))
            }
        }
    })
}

async fn start_processing_handler(
    mut read_output_rx: tokio::sync::mpsc::UnboundedReceiver<IncomingStreamData>,
    decoded_packet_tx: UnboundedSender<protobufs::FromRadio>,
) {
    debug!("Started message processing handler");

    let mut buffer = StreamBuffer::new(decoded_packet_tx);

    while let Some(message) = read_output_rx.recv().await {
        buffer.process_incoming_bytes(message);
    }

    debug!("Processing read_output_rx channel closed");
}

pub fn spawn_heartbeat_handler(
    cancellation_token: CancellationToken,
    write_input_tx: UnboundedSender<EncodedToRadioPacketWithHeader>,
) -> JoinHandle<Result<(), Error>> {
    let handle = start_heartbeat_handler(cancellation_token.clone(), write_input_tx);

    spawn(async move {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                debug!("Heartbeat handler cancelled");
                Ok(())
            }
            write_result = handle => {
                if let Err(e) = &write_result {
                    error!("Heartbeat handler unexpectedly terminated {e:?}");
                }
                write_result
            }
        }
    })
}

async fn start_heartbeat_handler(
    _cancellation_token: CancellationToken,
    write_input_tx: UnboundedSender<EncodedToRadioPacketWithHeader>,
) -> Result<(), Error> {
    debug!("Started heartbeat handler");

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(CLIENT_HEARTBEAT_INTERVAL)).await;

        let heartbeat_packet = protobufs::ToRadio {
            payload_variant: Some(protobufs::to_radio::PayloadVariant::Heartbeat(
                protobufs::Heartbeat::default(),
            )),
        };

        let mut buffer = Vec::new();
        match heartbeat_packet.encode(&mut buffer) {
            Ok(_) => (),
            Err(e) => {
                error!("Error encoding heartbeat packet: {:?}", e);
                continue;
            }
        };

        let packet_with_header = match format_data_packet(buffer.into()) {
            Ok(p) => p,
            Err(e) => {
                error!("Error formatting heartbeat packet: {:?}", e);
                continue;
            }
        };

        trace!("Sending heartbeat packet");

        if let Err(e) = write_input_tx.send(packet_with_header) {
            error!("Error writing heartbeat packet to stream: {:?}", e);
            return Err(Error::InternalStreamError(
                InternalStreamError::StreamWriteError {
                    source: Box::new(e),
                },
            ));
        }

        log::info!("Sent heartbeat packet");
    }

    // debug!("Heartbeat handler finished");

    // Return type should be never (!)
}
