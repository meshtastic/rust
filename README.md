# Meshtastic.rs

## Overview

Meshtastic.rs is a crate that allows you to interact with Meshtastic devices in Rust. This crate is designed
to be used on a desktop environment, and currently supports connecting to radios via USB serial and TCP.

This crate is designed to be used within the tokio asynchronous runtime.

<!-- [![Crates.io](https://img.shields.io/crates/v/crate_name)](https://crates.io/crates/crate_name)
[![Documentation](https://docs.rs/crate_name/badge.svg)](https://docs.rs/crate_name)
[![License](https://img.shields.io/crates/l/crate_name)](https://github.com/your_username/crate_name/blob/main/LICENSE) -->

## Installation

We are currently working to stablilize the API of this crate and publish it to crates.io. Until then, you can
install this crate using its git repository URL.

```toml
[dependencies]
meshtastic = { git = "https://github.com/meshtastic/rust" }
```

## Usage

This crate provides basic TCP and serial connection examples within the `/examples` directory. You can run
these examples using the following commands:

```bash
cargo run --example basic_tcp
cargo run --example basic_serial
```

### TCP Example

This example requires a Meshtastic with an exposed IP port, or a simulated radio via the Meshtastic Docker instance ([see here](https://meshtastic.org/docs/software/linux-native#usage-with-docker)).

```rust
/// This example connects to a TCP port on the radio, and prints out all received packets.
/// This can be used with a simulated radio via the Meshtastic Docker firmware image.
/// https://meshtastic.org/docs/software/linux-native#usage-with-docker

use std::io::{self, BufRead};

use meshtastic::connections::{helpers::generate_rand_id, stream_api::StreamApi};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream_api = StreamApi::new();

    println!("Enter the address of a TCP port to connect to, in the form \"IP:PORT\":");

    let stdin = io::stdin();
    let entered_address = stdin
        .lock()
        .lines()
        .next()
        .expect("Failed to find next line")
        .expect("Could not read next line");

    let tcp_stream = StreamApi::build_tcp_stream(entered_address).await?;
    let mut decoded_listener = stream_api.connect(tcp_stream).await;

    let config_id = generate_rand_id();
    stream_api.configure(config_id).await?;

    // This loop can be broken with ctrl+c, or by unpowering the radio.
    while let Some(decoded) = decoded_listener.recv().await {
        println!("Received: {:?}", decoded);
    }

    // Note that in this specific example, this will only be called when
    // the radio is disconnected, as the above loop will never exit.
    // Typically you would allow the user to manually kill the loop,
    // for example with tokio::select!.
    stream_api.disconnect().await?;

    Ok(())
}
```

### Serial Example

This example requires a powered and flashed Meshtastic radio connected to the host machine via a USB serial port.

```rust
/// This example connects to a radio via serial, and prints out all received packets.
/// This example requires a powered and flashed Meshtastic radio.
/// https://meshtastic.org/docs/supported-hardware

use std::io::{self, BufRead};

use meshtastic::{
    connections::{helpers::generate_rand_id, stream_api::StreamApi},
    utils::get_available_serial_ports,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream_api = StreamApi::new();

    let available_ports = get_available_serial_ports()?;
    println!("Available ports: {:?}", available_ports);
    println!("Enter the name of a port to connect to:");

    let stdin = io::stdin();
    let entered_port = stdin
        .lock()
        .lines()
        .next()
        .expect("Failed to find next line")
        .expect("Could not read next line");

    let serial_stream = StreamApi::build_serial_stream(entered_port, None, None, None)?;
    let mut decoded_listener = stream_api.connect(serial_stream).await;

    let config_id = generate_rand_id();
    stream_api.configure(config_id).await?;

    // This loop can be broken with ctrl+c, or by disconnecting
    // the attached serial port.
    while let Some(decoded) = decoded_listener.recv().await {
        println!("Received: {:?}", decoded);
    }

    // Note that in this specific example, this will only be called when
    // the radio is disconnected, as the above loop will never exit.
    // Typically you would allow the user to manually kill the loop,
    // for example with tokio::select!.
    stream_api.disconnect().await?;

    Ok(())
}
```

## Contributing

Contributions are welcome! If you find a bug or want to propose a new feature, please open an issue or submit a pull request.

## License

This project is licensed under the GPL-3.0 License.
