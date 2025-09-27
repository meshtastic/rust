//! This example connects to a TCP port on the radio and prints out all received packets.
//! This can be used with a simulated radio via the Meshtastic Docker firmware image.
//! https://meshtastic.org/docs/software/linux-native#usage-with-docker
extern crate meshtastic;

use std::io::{self, BufRead};
use std::time::SystemTime;

use meshtastic::api::StreamApi;
use meshtastic::utils;

/// Set up the logger to output to stdout  
/// **Note:** the invocation of this function is commented out in main by default.
#[allow(dead_code)]
fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_seconds(SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .chain(io::stdout())
        .apply()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Uncomment this to enable logging
    // setup_logger()?;

    let stream_api = StreamApi::new();

    println!("Enter the address of a TCP port to connect to, in the form \"IP:PORT\":");

    let stdin = io::stdin();
    let entered_address = stdin
        .lock()
        .lines()
        .next()
        .expect("Failed to find next line")
        .expect("Could not read next line");

    let tcp_stream = utils::stream::build_tcp_stream(entered_address).await?;
    let (mut decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;

    let config_id = utils::generate_rand_id();
    let stream_api = stream_api.configure(config_id).await?;

    // This loop can be broken with ctrl+c or by unpowering the radio.
    while let Some(decoded) = decoded_listener.recv().await {
        println!("Received: {:?}", decoded);
    }

    // Note that in this specific example, this will only be called when
    // the radio is disconnected, as the above loop will never exit.
    // Typically, you would allow the user to manually kill the loop,
    // for example, with tokio::select!.
    let _stream_api = stream_api.disconnect().await?;

    Ok(())
}
