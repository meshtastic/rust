use std::time::Duration;

use tokio_serial::{available_ports, SerialPort, SerialStream};

use crate::errors::Error;

// Constants declarations

pub const DEFAULT_SERIAL_BAUD: u32 = 115_200;
pub const DEFAULT_DTR_PIN: bool = true;
pub const DEFAULT_RTS_PIN: bool = false;

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
/// will be of the form /dev/ttyUSBx, while Windows ports will be of the form COMx.
/// * `baud_rate` - The baud rate of the serial port. Defaults to `115_200` if not passed.
/// * `dtr` - Asserts the "Data Terminal Ready" signal for the serial port if `true`.
/// Defaults to `true` if not passed.
/// * `rts` - Asserts the "Request To Send" signal for the serial port if `true`.
/// Defaults to `false` if not passed.
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
) -> Result<SerialStream, Error> {
    let builder = tokio_serial::new(port_name.clone(), baud_rate.unwrap_or(DEFAULT_SERIAL_BAUD))
        .flow_control(tokio_serial::FlowControl::Software)
        .timeout(Duration::from_millis(10));

    let mut serial_stream =
        tokio_serial::SerialStream::open(&builder).map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: format!("Error opening serial port \"{}\"", port_name).to_string(),
        })?;

    serial_stream
        .write_data_terminal_ready(dtr.unwrap_or(DEFAULT_DTR_PIN))
        .map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: "Failed to set DTR line".to_string(),
        })?;

    serial_stream
        .write_request_to_send(rts.unwrap_or(DEFAULT_RTS_PIN))
        .map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: "Failed to set RTS line".to_string(),
        })?;

    Ok(serial_stream)
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
/// let tcp_stream = utils::build_serial_stream("192.168.0.1:4403")?;
/// let decoded_listener = stream_api.connect(tcp_stream).await;
///
/// // Connect to a firmware Docker container
/// let tcp_stream = utils::build_serial_stream("localhost:4403")?;
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
pub async fn build_tcp_stream(address: String) -> Result<tokio::net::TcpStream, Error> {
    let connection_future = tokio::net::TcpStream::connect(address.clone());
    let timeout_duration = Duration::from_millis(3000);

    let stream = match tokio::time::timeout(timeout_duration, connection_future).await {
        Ok(stream) => stream.map_err(|e| Error::StreamBuildError {
            source: Box::new(e),
            description: format!("Failed to connect to {}", address).to_string(),
        })?,
        Err(e) => {
            return Err(Error::StreamBuildError{source:Box::new(e),description:format!(
                    "Timed out connecting to {}. Check that the radio is on, network is enabled, and the address is correct.",
                    address,
                )});
        }
    };

    Ok(stream)
}
