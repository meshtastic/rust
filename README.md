# Meshtastic.rs

## Overview

Meshtastic.rs is a crate that allows you to interact with Meshtastic devices in Rust. This crate is designed
to be used on a desktop environment and currently supports connecting to radios via USB serial and TCP.

This crate is designed to be used within the tokio asynchronous runtime.

[![Crates.io](https://img.shields.io/crates/v/meshtastic)](https://crates.io/crates/meshtastic)
[![Documentation](https://docs.rs/meshtastic/badge.svg)](https://docs.rs/meshtastic)
[![License](https://img.shields.io/crates/l/meshtastic)](https://github.com/meshtastic/rust/blob/main/LICENSE)

## Installation

You can add this crate to your project using the following command:

```shell
cargo add meshtastic
```

Alternatively, you can clone this repository to your own working directory:

```shell
git clone https://github.com/meshtastic/rust.git
```

Recursively clone our Git submodules by running:

```shell
git submodule update --init
```

## Usage

This crate provides basic TCP and serial connection examples within the `/examples` directory. You can run
these examples using the following commands:

```bash
cargo run --example basic_tcp
cargo run --example basic_serial
cargo run --features="bluetooth-le tokio" --example basic_ble
```

### TCP Example

This example requires a Meshtastic with an exposed IP port, or a simulated radio via the Meshtastic Docker instance ([see here](https://meshtastic.org/docs/software/linux-native#usage-with-docker)).

```rust
/// This example connects to a TCP port on the radio, and prints out all received packets.
/// This can be used with a simulated radio via the Meshtastic Docker firmware image.
/// https://meshtastic.org/docs/software/linux-native#usage-with-docker

use std::io::{self, BufRead};

use meshtastic::api::StreamApi;
use meshtastic::utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // This loop can be broken with ctrl+c, or by unpowering the radio.
    while let Some(decoded) = decoded_listener.recv().await {
        println!("Received: {:?}", decoded);
    }

    // Note that in this specific example, this will only be called when
    // the radio is disconnected, as the above loop will never exit.
    // Typically you would allow the user to manually kill the loop,
    // for example with tokio::select!.
    let _stream_api = stream_api.disconnect().await?;

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

use meshtastic::api::StreamApi;
use meshtastic::utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_api = StreamApi::new();

    let available_ports = utils::stream::available_serial_ports()?;
    println!("Available ports: {:?}", available_ports);
    println!("Enter the name of a port to connect to:");

    let stdin = io::stdin();
    let entered_port = stdin
        .lock()
        .lines()
        .next()
        .expect("Failed to find next line")
        .expect("Could not read next line");

    let serial_stream = utils::stream::build_serial_stream(entered_port, None, None, None)?;
    let (mut decoded_listener, stream_api) = stream_api.connect(serial_stream).await;

    let config_id = utils::generate_rand_id();
    let stream_api = stream_api.configure(config_id).await?;

    // This loop can be broken with ctrl+c, or by disconnecting
    // the attached serial port.
    while let Some(decoded) = decoded_listener.recv().await {
        println!("Received: {:?}", decoded);
    }

    // Note that in this specific example, this will only be called when
    // the radio is disconnected, as the above loop will never exit.
    // Typically you would allow the user to manually kill the loop,
    // for example with tokio::select!.
    let _stream_api = stream_api.disconnect().await?;

    Ok(())
}
```

### Bluetooth Low-energy (BLE) Example

This example requires a powered and flashed Meshtastic radio with BLE enabled. You need to pair it first using your operating system utilities. PIN might be needed.

> [!NOTE]
> You need `bluetooth-le` feature enabled for Bluetooth low-energy support.

```rust
/// This example connects via Bluetooth LE to the radio and prints out all received packets.
extern crate meshtastic;

use std::io::{self, BufRead};
use std::time::Duration;

use meshtastic::api::StreamApi;
use meshtastic::utils;
use meshtastic::utils::stream::BleId;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_api = StreamApi::new();

    println!("Enter the short name of a BLE device to connect to:");

    let stdin = io::stdin();
    let entered_name = stdin
        .lock()
        .lines()
        .next()
        .expect("Failed to find next line")
        .expect("Could not read next line");

    // You can also use `BleId::from_mac_address("AB")` instead of `BleId::from_name()`.
    let ble_stream =
        utils::stream::build_ble_stream(&BleId::from_name(&entered_name), Duration::from_secs(5))
            .await?;
    let (mut decoded_listener, stream_api) = stream_api.connect(ble_stream).await;

    let config_id = utils::generate_rand_id();
    let stream_api = stream_api.configure(config_id).await?;

    // This loop can be broken with ctrl+c, by disabling bluetooth or by turning off the radio.
    while let Some(decoded) = decoded_listener.recv().await {
        println!("Received: {:?}", decoded);
    }

    // Note that in this specific example, this will only be called when
    // the radio is disconnected, as the above loop will never exit.
    // Typically you would allow the user to manually kill the loop,
    // for example with tokio::select!.
    let _stream_api = stream_api.disconnect().await?;

    Ok(())
}
```

## Stats

![Alt](https://repobeats.axiom.co/api/embed/18c638d36dc51fd03acfe5c2e52979ad67b04bc9.svg "Repobeats analytics image")

## Contributing

Contributions are welcome! If you find a bug or want to propose a new feature, please open an issue or submit a pull request.

## License

This project is licensed under the GPL-3.0 License.
