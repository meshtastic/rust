use crate::errors::Error;
use crate::protobufs;
use log::{debug, error, trace};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::spawn;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::connections::helpers::format_data_packet;
use crate::connections::stream_buffer::StreamBuffer;

pub fn spawn_read_handler<R>(
    cancellation_token: CancellationToken,
    read_stream: R,
    read_output_tx: UnboundedSender<Vec<u8>>,
) -> JoinHandle<()>
where
    R: AsyncReadExt + Send + Unpin + 'static,
{
    let handle = start_read_handler(read_stream, read_output_tx.clone());

    spawn(async move {
        // Check for cancellation signal or handle termination
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                debug!("Read handler cancelled");
            }
            e = handle => {
                error!("Read handler unexpectedly terminated: {:#?}", e);
            }
        }
    })
}

async fn start_read_handler<R>(
    read_stream: R,
    read_output_tx: UnboundedSender<Vec<u8>>,
) -> Result<(), Error>
where
    R: AsyncReadExt + Send + Unpin + 'static,
{
    debug!("Started read handler");

    let mut read_stream = read_stream;

    loop {
        let mut buffer = [0u8; 1024];
        match read_stream.read(&mut buffer).await {
            Ok(0) => continue,
            Ok(n) => {
                trace!("Read {} bytes from stream", n);
                let data = buffer[..n].to_vec();
                trace!("Read data: {:?}", data);

                if let Err(e) = read_output_tx.send(data) {
                    error!("Failed to send data through channel");
                    return Err(Error::ChannelWriteFailure(e));
                }
            }

            // TODO check if port has fatally errored, and if so, tell UI
            Err(e) => {
                error!("Error reading from stream: {:?}", e);
                return Err(Error::StreamReadError {
                    source: Box::new(e),
                });
            }
        }
    }

    // trace!("Read handler finished");

    // Return type should be never (!)
}

pub fn spawn_write_handler<W>(
    cancellation_token: CancellationToken,
    write_stream: W,
    write_input_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
) -> JoinHandle<()>
where
    W: AsyncWriteExt + Send + Unpin + 'static,
{
    let handle = start_write_handler(cancellation_token.clone(), write_stream, write_input_rx);

    spawn(async move {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
              debug!("Write handler cancelled");
            }
            _ = handle => {
                error!("Write handler unexpectedly terminated");
            }
        }
    })
}

async fn start_write_handler<W>(
    _cancellation_token: CancellationToken,
    mut write_stream: W,
    mut write_input_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
) -> Result<(), Error>
where
    W: AsyncWriteExt + Send + Unpin + 'static,
{
    debug!("Started write handler");

    while let Some(message) = write_input_rx.recv().await {
        let packet_data = format_data_packet(message);
        trace!("Writing packet data: {:?}", packet_data);

        // // This might not be necessary
        // if cancellation_token.is_cancelled() {
        //     debug!("Write handler cancelled");
        //     break;
        // }

        if let Err(e) = write_stream.write(&packet_data).await {
            error!("Error writing to stream: {:?}", e);
            return Err(Error::StreamWriteError {
                source: Box::new(e),
            });
        }
    }

    debug!("Write handler finished");

    Ok(())
}

pub fn spawn_processing_handler(
    cancellation_token: CancellationToken,
    read_output_rx: UnboundedReceiver<Vec<u8>>,
    decoded_packet_tx: UnboundedSender<protobufs::FromRadio>,
) -> JoinHandle<()> {
    let handle = start_processing_handler(read_output_rx, decoded_packet_tx);

    spawn(async move {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
              debug!("Message processing handler cancelled");
            }
            _ = handle => {
              error!("Message processing handler unexpectedly terminated");
            }
        }
    })
}

async fn start_processing_handler(
    mut read_output_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    decoded_packet_tx: UnboundedSender<protobufs::FromRadio>,
) -> Result<(), Error> {
    trace!("Started message processing handler");

    let mut buffer = StreamBuffer::new(decoded_packet_tx);

    while let Some(message) = read_output_rx.recv().await {
        trace!("Processing {} bytes from radio", message.len());
        buffer.process_incoming_bytes(message);
    }

    trace!("Processing read_output_rx channel closed");

    Ok(())
}
