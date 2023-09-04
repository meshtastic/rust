use tokio_serial::available_ports;

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
