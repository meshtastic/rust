use tokio_serial::available_ports;

pub fn get_available_serial_ports() -> Result<Vec<String>, String> {
    let ports = available_ports()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|port| port.port_name)
        .collect();

    Ok(ports)
}
