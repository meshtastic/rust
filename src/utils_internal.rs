#[cfg(feature = "bluetooth-le")]
use crate::connections::ble_handler::BleHandler;
use crate::errors_internal::Error;
#[cfg(feature = "bluetooth-le")]
use btleplug::api::BDAddr;
#[cfg(feature = "bluetooth-le")]
use futures::stream::StreamExt;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use rand::{distr::StandardUniform, prelude::Distribution, Rng};
#[cfg(feature = "bluetooth-le")]
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio_serial::{available_ports, SerialPort, SerialStream};

use crate::connections::stream_api::StreamHandle;
use crate::connections::wrappers::encoded_data::{
    EncodedToRadioPacket, EncodedToRadioPacketWithHeader,
};

// Constants declarations

/// The default baud rate of incoming serial connections created by the `build_serial_stream` method.
pub const DEFAULT_SERIAL_BAUD: u32 = 115_200;

/// The default pin state of the DTR pin of incoming serial connections created by the `build_serial_stream` method.
pub const DEFAULT_DTR_PIN_STATE: bool = true;

/// The default pin state of the RTS pin of incoming serial connections created by the `build_serial_stream` method.
pub const DEFAULT_RTS_PIN_STATE: bool = false;

/// A helper method that uses the `tokio_serial` crate to list the names of all
/// available serial ports on the system. This method is intended to be used
/// to select a valid serial port, then to pass that port name to the `connect`
/// method.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// A result that resolves to a vector of strings, where each string is the name
/// of a serial port on the system.
///
/// # Examples
///
/// ```
/// let serial_ports = utils::available_serial_ports().unwrap();
/// println!("Available serial ports: {:?}", serial_ports);
/// ```
///
/// # Errors
///
/// Fails if the method fails to fetch available serial ports.
///
/// # Panics
///
/// None
///
pub fn available_serial_ports() -> Result<Vec<String>, tokio_serial::Error> {
    let ports = available_ports()?
        .into_iter()
        .map(|port| port.port_name)
        .collect();

    Ok(ports)
}

/// A helper method that uses the `tokio_serial` crate to build a serial stream
/// that is compatible with the `StreamApi` API. This requires that the stream
/// implements `AsyncReadExt + AsyncWriteExt` traits.
///
/// This method is intended to be used to create a `SerialStream` instance, which is
/// then passed into the `StreamApi::connect` method.
///
/// # Arguments
///
/// * `port_name` - The system-specific name of the serial port to open. Unix ports
///   will be of the form /dev/ttyUSBx, while Windows ports will be of the form COMx.
/// * `baud_rate` - The baud rate of the serial port. Defaults to `115_200` if not passed.
/// * `dtr` - Asserts the "Data Terminal Ready" signal for the serial port if `true`.
///   Defaults to `true` if not passed.
/// * `rts` - Asserts the "Request To Send" signal for the serial port if `true`.
///   Defaults to `false` if not passed.
///
/// # Returns
///
/// Returns a result that resolves to a `tokio_serial::SerialStream` instance, or
/// a `String` error message if the stream could not be created.
///
/// # Examples
///
/// ```
/// // Accept default parameters
/// let serial_stream = utils::build_serial_stream("/dev/ttyUSB0".to_string(), None, None, None)?;
/// let decoded_listener = stream_api.connect(serial_stream).await;
///
/// // Specify all parameters
/// let serial_stream = utils::build_serial_stream("/dev/ttyUSB0".to_string(), Some(115_200), Some(true), Some(false))?;
/// let decoded_listener = stream_api.connect(serial_stream).await;
/// ```
///
/// # Errors
///
/// Will return a `String` error message in the event the stream could not be opened, or
/// if the `dtr` and `rts` signals fail to assert.
///
/// # Panics
///
/// None
///
pub fn build_serial_stream(
    port_name: String,
    baud_rate: Option<u32>,
    dtr: Option<bool>,
    rts: Option<bool>,
) -> Result<StreamHandle<SerialStream>, Error> {
    let builder = tokio_serial::new(port_name.clone(), baud_rate.unwrap_or(DEFAULT_SERIAL_BAUD))
        .flow_control(tokio_serial::FlowControl::None)
        .timeout(Duration::from_millis(10));

    let mut serial_stream =
        tokio_serial::SerialStream::open(&builder).map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: format!("Error opening serial port \"{port_name}\"").to_string(),
        })?;

    serial_stream
        .write_data_terminal_ready(dtr.unwrap_or(DEFAULT_DTR_PIN_STATE))
        .map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: "Failed to set DTR line".to_string(),
        })?;

    serial_stream
        .write_request_to_send(rts.unwrap_or(DEFAULT_RTS_PIN_STATE))
        .map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: "Failed to set RTS line".to_string(),
        })?;

    Ok(StreamHandle::from_stream(serial_stream))
}

/// A helper method that uses the `tokio` crate to build a TCP stream
/// that is compatible with the `StreamApi` API. This requires that the stream
/// implements `AsyncReadExt + AsyncWriteExt` traits.
///
/// This method is intended to be used to create a `TcpStream` instance, which is
/// then passed into the `StreamApi::connect` method.
///
/// # Arguments
///
/// * `address` - The full TCP address of the device, including the port.
///
/// # Returns
///
/// Returns a result that resolves to a `tokio::net::TcpStream` instance, or
/// a `String` error message if the stream could not be created.
///
/// # Examples
///
/// ```
/// // Connect to a radio
/// let tcp_stream = utils::build_tcp_stream("192.168.0.1:4403").await?;
/// let decoded_listener = stream_api.connect(tcp_stream).await;
///
/// // Connect to a firmware Docker container
/// let tcp_stream = utils::build_tcp_stream("localhost:4403").await?;
/// let decoded_listener = stream_api.connect(tcp_stream).await;
/// ```
///
/// # Errors
///
/// Will return a `String` error message in the event that the radio refuses the connection,
/// or if the specified address is invalid.
///
/// # Panics
///
/// None
///
pub async fn build_tcp_stream(
    address: String,
) -> Result<StreamHandle<tokio::net::TcpStream>, Error> {
    let connection_future = tokio::net::TcpStream::connect(address.clone());
    let timeout_duration = Duration::from_millis(3000);

    let stream = match tokio::time::timeout(timeout_duration, connection_future).await {
        Ok(stream) => stream.map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: format!("Failed to connect to {address}"),
        })?,
        Err(e) => {
            return Err(Error::StreamBuildError{
                source: Box::new(e),
                description: format!(
                    "Timed out connecting to {address}. Check that the radio is on, network is enabled, and the address is correct."
                )});
        }
    };

    Ok(StreamHandle::from_stream(stream))
}

/// A helper method to list the names of all reachable Meshtastic Bluetooth radios.
///
/// This method is intended to be used to select a valid Bluetooth radio, then to pass that device
/// MAC address to the `build_ble_stream` method.
///
/// # Arguments
///
/// `scan_duration` - Duration of a Bluetooth LE scan for devices
///
/// # Returns
///
/// A vector of Bluetooth devices identified by a MAC address and optionally also a name.
///
/// # Examples
///
/// ```
/// let ble_devices = utils::available_ble_devices().await.unwrap();
/// println!("Available Meshtastic BLE devices: {:?}", ble_devices);
/// let stream = build_ble_stream(&ble_devices[0].1.into(), Duration::from_secs(10)).await;
/// ```
///
/// # Errors
///
/// Fails if the Blueetooth scan fails.
#[cfg(feature = "bluetooth-le")]
pub async fn available_ble_devices(
    scan_duration: Duration,
) -> Result<Vec<(Option<String>, BDAddr)>, Error> {
    BleHandler::available_ble_devices(scan_duration).await
}

/// A helper method that uses the `btleplug` and `tokio` crates to build a BLE stream
/// that is compatible with the `StreamApi` API. This requires that the stream
/// implements `AsyncReadExt + AsyncWriteExt` traits.
///
/// This method is intended to be used to create a `DuplexStream` instance, which is
/// then passed into the `StreamApi::connect` method.
///
/// # Arguments
///
/// * `ble_id` - Name or MAC address of a BLE device to connect to.
/// * `scan_duration` - The duration of the BLE scan.
///
/// # Returns
///
/// Returns a result that resolves to a `tokio::io::DuplexStream` instance, or
/// an error if the stream could not be created.
///
/// # Examples
///
/// ```
/// // Connect to a radio, identified by its MAC address
/// let duplex_stream = utils::build_ble_stream(
///     &BleId::from_mac_address("E3:44:4E:18:F7:A4").unwrap(),
///     Duration::from_secs(5),
/// )
/// .await?;
/// let decoded_listener = stream_api.connect(duplex_stream).await;
/// ```
///
/// # Errors
///
/// Will return an instance of `Error` in the event that the radio refuses the connection, it
/// cannot be found or if the specified address is invalid.
///
/// # Panics
///
/// None
///
#[cfg(feature = "bluetooth-le")]
pub async fn build_ble_stream(
    ble_id: &crate::connections::ble_handler::BleId,
    scan_duration: Duration,
) -> Result<StreamHandle<DuplexStream>, Error> {
    use crate::{
        connections::ble_handler::{AdapterEvent, RadioMessage},
        errors_internal::InternalStreamError,
    };
    let ble_handler = BleHandler::new(ble_id, scan_duration).await?;
    // `client` will be returned to the user, server is the opposite end of the channel and it's
    // directly connected to a `BleHandler`.
    let (client, mut server) = tokio::io::duplex(1024);
    let handle = tokio::spawn(async move {
        let duplex_write_error_fn = |e| {
            Error::InternalStreamError(InternalStreamError::StreamWriteError {
                source: Box::new(e),
            })
        };
        let mut read_messages_count = ble_handler.read_fromnum().await?;
        let mut buf = [0u8; 1024];
        if let Ok(len) = server.read(&mut buf).await {
            ble_handler.write_to_radio(&buf[..len]).await?
        }
        loop {
            match ble_handler.read_from_radio().await? {
                RadioMessage::Eof => break,
                RadioMessage::Packet(packet) => {
                    server
                        .write(packet.data())
                        .await
                        .map_err(duplex_write_error_fn)?;
                }
            }
        }

        let mut notification_stream = ble_handler.notifications().await?;
        let mut adapter_events = ble_handler.adapter_events().await?;
        loop {
            // Note: the following `tokio::select` is only half-duplex on the BLE radio. While we
            // are reading from the radio, we are not writing to it and vice versa. However, BLE is
            // a half-duplex technology, so we wouldn't gain much with a full duplex solution
            // anyway.
            tokio::select!(
                // Data from device, forward it to the user
                notification = notification_stream.next() => {
                    let avail_msg_count = notification.ok_or(InternalStreamError::Eof)?;
                    for _ in read_messages_count..avail_msg_count {
                        if let RadioMessage::Packet(packet) = ble_handler.read_from_radio().await? {
                            server.write(packet.data()).await.map_err(duplex_write_error_fn)?;
                        }
                    }
                    read_messages_count = avail_msg_count;
                },
                // Data from user, forward it to the device
                from_server = server.read(&mut buf) => {
                    let len = from_server.map_err(duplex_write_error_fn)?;
                    ble_handler.write_to_radio(&buf[..len]).await?;
                },
                event = adapter_events.next() => {
                    if Some(AdapterEvent::Disconnected) == event {
                        log::error!("BLE disconnected");
                        Err(InternalStreamError::ConnectionLost)?
                    }
                }
            );
        }
    });

    Ok(StreamHandle {
        stream: client,
        join_handle: Some(handle),
    })
}

/// A helper method to generate random numbers using the `rand` crate.
///
/// This method is intended to be used to generate random id values. This method
/// is generic, and will generate a random value within the range of the passed generic type.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// A random value of the passed generic type.
///
/// # Examples
///
/// ```
/// let packet_id = utils::generate_rand_id::<u32>();
/// println!("Generated random id: {}", packet_id);
/// ```
///
/// # Errors
///
/// None
///
/// # Panics
///
/// None
///
pub fn generate_rand_id<T>() -> T
where
    StandardUniform: Distribution<T>,
{
    let mut rng = rand::rng();
    rng.random()
}

/// A helper function that takes a vector of bytes (u8) representing an encoded packet, and
/// reuturns a new vector of bytes representing the encoded packet with the required packet
/// header attached.
///
/// The header format is shown below:
///
/// ```text
/// | 0x94 (1 byte) | 0xc3 (1 byte) | MSB (1 byte) | LSB (1 byte) | DATA ((MSB << 8) | LSB bytes) |
/// ```
///
/// * `0x94` and `0xc3` are the magic bytes that are required to be at the start of every packet.
/// * `(MSB << 8) | LSB` represents the length of the packet data (`DATA`) in bytes.
/// * `DATA` is the encoded packet data that is passed to this function.
///
/// # Arguments
///
/// * `data` - A vector of bytes representing the encoded packet data.
///
/// # Returns
///
/// A vector of bytes representing the encoded packet with the required packet header attached.
///
/// # Examples
///
/// ```
/// let packet = protobufs::ToRadio { payload_variant };
///
/// let mut packet_buf: Vec<u8> = vec![];
/// packet.encode::<Vec<u8>>(&mut packet_buf)?;
///
/// let packet_buf_with_header = utils::format_data_packet(packet_buf);
/// ```
///
/// # Errors
///
/// None
///
/// # Panics
///
/// None
///
pub fn format_data_packet(
    packet: EncodedToRadioPacket,
) -> Result<EncodedToRadioPacketWithHeader, Error> {
    let data = packet.data();
    if data.len() >= 1 << 16 {
        return Err(Error::InvalidaDataSize {
            data_length: data.len(),
        });
    }
    let [lsb, msb, ..] = data.len().to_le_bytes();
    let magic_buffer = [0x94, 0xc3, msb, lsb];

    Ok([&magic_buffer, data].concat().into())
}

/// A helper function that takes a vector of bytes (u8) representing an encoded packet with a 4-byte header,
/// and returns a new vector of bytes representing the encoded packet with the packet header removed.
///
/// The header format is shown below:
///
/// ```text
/// | 0x94 (1 byte) | 0xc3 (1 byte) | MSB (1 byte) | LSB (1 byte) | DATA ((MSB << 8) | LSB bytes) |
/// ```
///
/// * `0x94` and `0xc3` are the magic bytes that are required to be at the start of every packet.
/// * `(MSB << 8) | LSB` represents the length of the packet data (`DATA`) in bytes.
/// * `DATA` is the encoded packet data that is passed to this function.
///
/// # Arguments
///
/// * `packet` - A vector of bytes representing the encoded packet with the packet header attached.
///
/// # Returns
///
/// A vector of bytes representing the encoded packet with the packet header removed.
///
/// # Examples
///
/// ```
/// let packet = protobufs::ToRadio { payload_variant };
///
/// let mut packet_buf: Vec<u8> = vec![];
/// packet.encode::<Vec<u8>>(&mut packet_buf)?;
///
/// let packet_buf_with_header = utils::format_data_packet(packet_buf);
/// let stripped_packet_buf = utils::strip_data_packet_header(packet_buf_with_header)?;
///
/// assert_eq!(packet_buf, stripped_packet_buf.data_vec());
/// ```
///
/// # Errors
///
/// Will return an `Error::InsufficientPacketBufferLength` error if the passed packet buffer
/// is less than 4 bytes in length.
///
/// # Panics
///
/// None
///
pub fn strip_data_packet_header(
    packet: EncodedToRadioPacketWithHeader,
) -> Result<EncodedToRadioPacket, Error> {
    let data = packet.data_vec();

    let stripped_data = match data.get(4..) {
        Some(data) => data,
        None => return Err(Error::InsufficientPacketBufferLength { packet }),
    };

    Ok(stripped_data.into())
}

/// A helper function that returns the number of seconds since the unix epoch.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// A u32 representing the number of seconds since the unix epoch.
///
/// # Examples
///
/// ```
/// let epoch_secs = utils::current_epoch_secs_u32();
/// println!("Seconds since unix epoch: {}", epoch_secs);
/// ```
///
/// # Errors
///
/// None
///
/// # Panics
///
/// Panics if the current time is before the unix epoch. This should never happen.
///
/// Panics if the number of seconds since the unix epoch is greater than u32::MAX.
/// This will require a complete rewrite of the core Meshtastic infrastructure,
/// so this isn't something you should worry about.
///
pub fn current_epoch_secs_u32() -> u32 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Could not get time since unix epoch")
        .as_secs()
        .try_into()
        .expect("Could not convert u128 to u32")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_empty_packet() {
        let data = vec![];
        let serial_data = format_data_packet(data.into());

        assert_eq!(serial_data.unwrap().data(), vec![0x94, 0xc3, 0x00, 0x00]);
    }

    #[test]
    fn valid_non_empty_packet() {
        let data = vec![0x00, 0xff, 0x88];
        let serial_data = format_data_packet(data.into());

        assert_eq!(
            serial_data.unwrap().data(),
            vec![0x94, 0xc3, 0x00, 0x03, 0x00, 0xff, 0x88]
        );
    }

    #[test]
    fn valid_large_packet() {
        let data = vec![0x00; 0x100];
        let serial_data = format_data_packet(data.into());

        assert_eq!(
            serial_data.unwrap().data()[..4],
            vec![0x94, 0xc3, 0x01, 0x00]
        );
    }

    #[test]
    fn invalid_too_large_packet() {
        let data = vec![0x00; 0x10000];
        let serial_data = format_data_packet(data.into());

        assert!(serial_data.is_err());
    }
}
