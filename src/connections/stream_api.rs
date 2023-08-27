use crate::protobufs;
use log::trace;
use prost::Message;
use std::{fmt::Display, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_serial::{SerialPort, SerialStream};
use tokio_util::sync::CancellationToken;

use super::{
    handlers,
    helpers::{generate_rand_id, get_current_time_u32},
    PacketDestination, PacketRouter,
};

pub const DEFAULT_SERIAL_BAUD: u32 = 115_200;
pub const DEFAULT_DTR_PIN: bool = true;
pub const DEFAULT_RTS_PIN: bool = false;

#[derive(Debug, Default)]
pub struct StreamApi {
    write_input_tx: Option<UnboundedSender<Vec<u8>>>,

    read_handle: Option<JoinHandle<()>>,
    write_handle: Option<JoinHandle<()>>,
    processing_handle: Option<JoinHandle<()>>,

    cancellation_token: Option<CancellationToken>,
}

// Helper functions for building `AsyncReadExt + AsyncWriteExt` streams

impl StreamApi {
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
    /// let serial_stream = StreamApi::build_serial_stream("/dev/ttyUSB0".to_string(), None, None, None)?;
    /// let decoded_listener = stream_api.connect(serial_stream).await;
    ///
    /// // Specify all parameters
    /// let serial_stream = StreamApi::build_serial_stream("/dev/ttyUSB0".to_string(), Some(115_200), Some(true), Some(false))?;
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
    ) -> Result<SerialStream, String> {
        let builder =
            tokio_serial::new(port_name.clone(), baud_rate.unwrap_or(DEFAULT_SERIAL_BAUD))
                .flow_control(tokio_serial::FlowControl::Software)
                .timeout(Duration::from_millis(10));

        let mut serial_stream = tokio_serial::SerialStream::open(&builder).map_err(|e| {
            format!("Error opening serial port \"{}\": {:?}", port_name, e).to_string()
        })?;

        serial_stream
            .write_data_terminal_ready(dtr.unwrap_or(DEFAULT_DTR_PIN))
            .map_err(|e| format!("Error setting DTR: {:?}", e).to_string())?;

        serial_stream
            .write_request_to_send(rts.unwrap_or(DEFAULT_RTS_PIN))
            .map_err(|e| format!("Error setting RTS: {:?}", e).to_string())?;

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
    /// let tcp_stream = StreamApi::build_serial_stream("192.168.0.1:4403")?;
    /// let decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// // Connect to a firmware Docker container
    /// let tcp_stream = StreamApi::build_serial_stream("localhost:4403")?;
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
    pub async fn build_tcp_stream(address: String) -> Result<tokio::net::TcpStream, String> {
        let connection_future = tokio::net::TcpStream::connect(address.clone());
        let timeout_duration = Duration::from_millis(3000);

        let stream = match tokio::time::timeout(timeout_duration, connection_future).await {
            Ok(stream) => stream.map_err(|e| e.to_string())?,
            Err(e) => {
                return Err(format!(
                    "Timed out connecting to {} with error \"{}.\" Check that the radio is on, network is enabled, and the address is correct.",
                    address,
                    e.to_string()
                ));
            }
        };

        Ok(stream)
    }
}

// Internal helper functions

impl StreamApi {
    async fn send_packet<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        byte_data: Vec<u8>,
        port_num: protobufs::PortNum,
        destination: PacketDestination,
        channel: u32, // TODO this should be scoped to 0-7
        want_ack: bool,
        want_response: bool,
        echo_response: bool,
        reply_id: Option<u32>,
        emoji: Option<u32>,
    ) -> Result<(), String> {
        // let own_node_id: u32 = self.my_node_info.as_ref().unwrap().my_node_num;
        let own_node_id: u32 = packet_router.get_source_node_id();

        let packet_destination: u32 = match destination {
            PacketDestination::Local => own_node_id,
            PacketDestination::Broadcast => 0xffffffff,
            PacketDestination::Node(id) => id,
        };

        let mut packet = protobufs::MeshPacket {
            payload_variant: Some(protobufs::mesh_packet::PayloadVariant::Decoded(
                protobufs::Data {
                    portnum: port_num as i32,
                    payload: byte_data,
                    want_response,
                    reply_id: reply_id.unwrap_or(0),
                    emoji: emoji.unwrap_or(0),
                    dest: 0,       // TODO change this
                    request_id: 0, // TODO change this
                    source: 0,     // TODO change this
                },
            )),
            rx_time: 0,   // * not transmitted
            rx_snr: 0.0,  // * not transmitted
            hop_limit: 0, // * not transmitted
            priority: 0,  // * not transmitted
            rx_rssi: 0,   // * not transmitted
            delayed: 0,   // * not transmitted
            from: own_node_id,
            to: packet_destination,
            id: generate_rand_id(),
            want_ack,
            channel,
        };

        if echo_response {
            packet.rx_time = get_current_time_u32();
            packet_router
                .handle_mesh_packet(packet.clone())
                .map_err(|e| e.to_string())?;
        }

        let to_radio = protobufs::ToRadio {
            payload_variant: Some(protobufs::to_radio::PayloadVariant::Packet(packet)),
        };

        let mut packet_buf: Vec<u8> = vec![];
        to_radio
            .encode::<Vec<u8>>(&mut packet_buf)
            .map_err(|e| e.to_string())?;

        self.send_raw(packet_buf).await?;

        Ok(())
    }

    // TODO want a way to force connection before configuration
    async fn send_raw(&mut self, data: Vec<u8>) -> Result<(), String> {
        let channel = self
            .write_input_tx
            .as_ref()
            .ok_or("Could not send message to write channel")
            .map_err(|e| e.to_string())?;

        channel.send(data).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// Public connection management API

impl StreamApi {
    /// A method to create an unconfigured instance of the `StreamApi` struct.
    ///
    /// # Arguments
    ///
    /// None
    ///
    /// # Returns
    ///
    /// Returns an instance of the `StreamApi` struct with default values for all private fields.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
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
    pub fn new() -> Self {
        Self::default()
    }

    /// A method to connect to a radio via a provided stream. This method is generic,
    /// and requires the `stream` parameter to implement the `AsyncReadExt + AsyncWriteExt`.
    ///
    /// This method is used to configure a `StreamApi` instance to communicate with a radio,
    /// usually via a serial port or a TCP connection. The user is expected to call the `connect`
    /// method before calling the `configure` method. This method will spawn read and write worker
    /// threads that will manage communication with the radio, as well as a message processing
    /// thread. This method will also initialize a cancellation token used in the `disconnect` method.
    ///
    /// # Arguments
    ///
    /// * `stream` - A generic stream that implements the `AsyncReadExt + AsyncWriteExt` traits.
    ///
    /// # Returns
    ///
    /// Returns an `UnboundedReceiver` that is used to receive decoded `FromRadio` packets.
    ///
    /// # Examples
    ///
    /// ```
    /// // Example 1: Connect to a serial port
    /// let serial_stream = build_serial_stream("/dev/ttyUSB0".to_string(), None, None, None)?;
    /// let decoded_listener = stream_api.connect(serial_stream).await;
    ///
    /// // Example 2: Connect to a TCP port
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let decoded_listener = stream_api.connect(tcp_stream).await;
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
    pub async fn connect<S>(&mut self, stream: S) -> UnboundedReceiver<protobufs::FromRadio>
    where
        S: AsyncReadExt + AsyncWriteExt + Send + 'static,
    {
        // Create message channels

        let (write_input_tx, write_input_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let (read_output_tx, read_output_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let (decoded_packet_tx, decoded_packet_rx) =
            tokio::sync::mpsc::unbounded_channel::<protobufs::FromRadio>();

        // Spawn worker threads with kill switch

        let (read_stream, write_stream) = tokio::io::split(stream);
        let cancellation_token = CancellationToken::new();

        self.read_handle = Some(handlers::spawn_read_handler(
            cancellation_token.clone(),
            read_stream,
            read_output_tx,
        ));

        self.write_handle = Some(handlers::spawn_write_handler(
            cancellation_token.clone(),
            write_stream,
            write_input_rx,
        ));

        self.processing_handle = Some(handlers::spawn_processing_handler(
            cancellation_token.clone(),
            read_output_rx,
            decoded_packet_tx,
        ));

        // Persist channels and kill switch to struct

        self.write_input_tx = Some(write_input_tx);
        self.cancellation_token = Some(cancellation_token);

        // Return channel for receiving decoded packets

        decoded_packet_rx
    }

    /// A method to disconnect from a radio. This method will close all channels and
    /// join all worker threads. If connected via serial or TCP, this will also trigger
    /// the radio to terminate its current connection.
    ///
    /// This method can only be called after the `connect` method has been called.
    ///
    /// # Arguments
    ///
    /// None
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating whether or not the disconnection was successful.
    /// If not successful, the `Err(String)` variant will contain information on the failure.
    ///
    /// # Examples
    ///
    /// ```
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// // Process packets from the `decoded_listener` channel
    ///
    /// stream_api.disconnect().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Will fail if any of the worker threads fail to join.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn disconnect(&mut self) -> Result<(), String> {
        // Tell worker threads to shut down
        if let Some(token) = self.cancellation_token.take() {
            token.cancel();
        }

        // Close channels, which will kill worker threads

        self.write_input_tx = None;

        // Close worker threads

        if let Some(serial_read_handle) = self.read_handle.take() {
            serial_read_handle
                .await
                .map_err(|_e| "Error joining serial_read_handle".to_string())?;
        }

        if let Some(serial_write_handle) = self.write_handle.take() {
            serial_write_handle
                .await
                .map_err(|_e| "Error joining serial_write_handle".to_string())?;
        }

        if let Some(processing_handle) = self.processing_handle.take() {
            processing_handle
                .await
                .map_err(|_e| "Error joining message_processing_handle".to_string())?;
        }

        trace!("TCP handlers fully disconnected");

        Ok(())
    }

    /// This method is used to trigger the transmission of the current state of the
    /// radio, as well as to subscribe to future `FromRadio` mesh packets. This method
    /// must be called after the `connect` method.
    ///
    /// This method triggers a `WantConfigId` packet to be sent to the radio containing
    /// an arbitrary configuration identifier. This will tell the radio that the client
    /// wants to connect. The radio will respond by sending its current configuration,
    /// module configuration, and channel configuration. The radio will indicate that it
    /// has finished transmission by sending a `ConfigComplete` packet, which will contain
    /// the same configuration identifier that was sent in the `WantConfigId` packet. Tracking
    /// whether or not configuration completes successfully is not handled by this method.
    ///
    /// Once a radio connection has been configured, the radio will send all future packets
    /// it receives through the decoded packet channel. This will continue until the
    /// `disconnect` method is called.
    ///
    /// # Arguments
    ///
    /// * `config_id` - A randomly generated configuration ID that will be used
    /// to check that the configuration process has completed.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating whether or not the configuration was successful.
    /// The configuration will fail if the `WantConfigId` packet fails to send.
    ///
    /// # Examples
    ///
    /// ```
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// let config_id = gen_random_id();
    /// stream_api.configure(config_id).await?;
    ///
    /// while let Some(packet) = decoded_listener.recv().await {
    ///     // Process packets
    /// }
    ///
    /// stream_api.disconnect().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the `WantConfigId` packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn configure(&mut self, config_id: u32) -> Result<(), String> {
        let to_radio = protobufs::ToRadio {
            payload_variant: Some(protobufs::to_radio::PayloadVariant::WantConfigId(config_id)),
        };

        let packet_buf = to_radio.encode_to_vec();
        self.send_raw(packet_buf).await?;

        Ok(())
    }
}

// Public node management API

impl StreamApi {
    /// Sends the specified text content over the mesh.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `text` - A `String` containing the text to send.
    /// * `destination` - A `PacketDestination` enum that specifies the destination of the packet.
    /// * `want_ack` - A `bool` that specifies whether or not the radio should wait for acknowledgement
    /// from other nodes on the mesh.
    /// * `channel` - A `u32` that specifies the message channel to send the packet on [0..7).
    ///
    /// # Returns
    ///
    /// A result indicating whether the packet was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let _decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// stream_api.send_text(packet_router, "Hello world!".to_string(), PacketDestination::Broadcast, true, 0).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn send_text<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        text: String,
        destination: PacketDestination,
        want_ack: bool,
        channel: u32,
    ) -> Result<(), String> {
        let byte_data = text.into_bytes();

        self.send_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::TextMessageApp,
            destination,
            channel,
            want_ack,
            false,
            true,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    /// Sends the specified `Waypoint` over the mesh.
    ///
    /// If the specified `Waypoint` struct has an `id` field of `0`, this method will generate
    /// a random id for the waypoint and update the struct before sending. This is because 0
    /// is an invalid waypoint ID.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `wypoint` - An instance of the `Waypoint` struct to send.
    /// * `destination` - A `PacketDestination` enum that specifies the destination of the packet.
    /// * `want_ack` - A `bool` that specifies whether or not the radio should wait for acknowledgement
    /// from other nodes on the mesh.
    /// * `channel` - A `u32` that specifies the message channel to send the packet on [0..7).
    ///
    /// # Returns
    ///
    /// A result indicating whether the packet was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let _decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// let waypoint = crate::protobufs::Waypoint { ... };
    /// stream_api.send_waypoint(packet_router, waypoint, PacketDestination::Broadcast, true, 0).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn send_waypoint<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        waypoint: crate::protobufs::Waypoint,
        destination: PacketDestination,
        want_ack: bool,
        channel: u32,
    ) -> Result<(), String> {
        let mut waypoint = waypoint;

        // Waypoint with ID of zero denotes a new waypoint; check whether to generate its ID on backend
        if waypoint.id == 0 {
            waypoint.id = generate_rand_id();
        }
        let byte_data = waypoint.encode_to_vec();

        self.send_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::WaypointApp,
            destination,
            channel,
            want_ack,
            false,
            true,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    /// Updates the configuration of the radio to the specified configuration.
    ///
    /// This method takes in an enum with variants for each configuration type. In the
    /// examples below, the method will update the position configuration for the
    /// connected radio. To update multiple configuration fields at once, see the
    /// `set_local_config` method.
    ///
    /// **Note:** The radio will restart after updating the module configuration, which
    /// will disconnect the `StreamApi` instance.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `config` - An instance of the `Config` struct to update the radio with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the config was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let _decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// let config_update = crate::protobufs::config::PositionConfig { ... };
    /// stream_api.update_config(packet_router, config_update).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn update_config<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        config: protobufs::Config,
    ) -> Result<(), String> {
        let config_packet = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::SetConfig(config)),
        };

        let byte_data = config_packet.encode_to_vec();

        self.send_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::AdminApp,
            PacketDestination::Local,
            0,
            true,
            true,
            false,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    /// Updates the module configuration of the radio to the specified configuration.
    ///
    /// This method takes in an enum with variants for each module configuration type. In the
    /// examples below, the method will update the MQTT configuration for the
    /// connected radio. To update multiple module configuration fields at once, see the
    /// `set_local_module_config` method.
    ///
    /// **Note:** The radio will restart after updating the module configuration, which
    /// will disconnect the `StreamApi` instance.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `module_config` - An instance of the `ModuleConfig` struct to update the radio with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the module config was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let _decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// let module_config_update = crate::protobufs::module_config::MqttConfig { ... };
    /// stream_api.update_module_config(packet_router, module_config_update).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn update_module_config<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        module_config: protobufs::ModuleConfig,
    ) -> Result<(), String> {
        let module_config_packet = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::SetModuleConfig(
                module_config,
            )),
        };

        let byte_data = module_config_packet.encode_to_vec();

        self.send_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::AdminApp,
            PacketDestination::Local,
            0,
            true,
            true,
            false,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    /// Updates the message channel configuration of the radio to the specified configuration.
    ///
    /// This method takes in an enum with variants for each channel configuration type. In the
    /// examples below, the method will update the configuration for an arbitrary channel in the
    /// connected radio. To update multiple channels at once, see the `set_local_module_config` method.
    ///
    /// **Note:** The radio will **NOT** restart after updating the channel configuration.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `channel_config` - An instance of the `Channel` struct to update the radio with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the channel config was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let _decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// let channel_config_update = crate::protobufs::Channel { id: 1, ... }
    /// stream_api.update_channel_config(packet_router, channel_config_update).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn update_channel_config<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        channel_config: protobufs::Channel,
    ) -> Result<(), String> {
        // Tell device to update channels

        let channel_packet = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::SetChannel(
                channel_config,
            )),
        };

        let byte_data = channel_packet.encode_to_vec();

        self.send_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::AdminApp,
            PacketDestination::Local,
            0,
            true,
            true,
            false,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    /// Updates information on the user of the connected radio. This information is periodically
    /// transmitted out into the mesh to allow other nodes to identify the owner of the radio.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `user` - An instance of the `User` struct to update the radio user with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the new `User` information was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string())?;
    /// let _decoded_listener = stream_api.connect(tcp_stream).await;
    ///
    /// let new_user = crate::protobufs::User { ... };
    /// stream_api.update_user(packet_router, new_user).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn update_user<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        user: protobufs::User,
    ) -> Result<(), String> {
        let user_packet = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::SetOwner(user)),
        };

        let byte_data = user_packet.encode_to_vec();

        self.send_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::AdminApp,
            PacketDestination::Local,
            0,
            true,
            true,
            false,
            None,
            None,
        )
        .await?;

        Ok(())
    }

    /// A method to tell the radio to begin a bulk configuration update.
    ///
    /// This method is intended to be used to batch multiple configuration updates into a single
    /// transaction on the radio. This is meant to avoid needing to wait for the radio
    /// to restart multiple times when updating multiple configuration fields.
    ///
    /// After calling this method, any calls to `update_config`, `update_module_config`, or `update_channel_config`
    /// will be buffered on the radio until the `commit_config_transaction` method is called. This will
    /// then trigger a radio restart, and the buffered configuration updates will be applied.
    ///
    /// **Note:** It is not supported to batch configuration, module configuration,
    /// and channel configuration updates together. These must be done in separate transactions.
    /// This is a limitation of the current firmware.
    ///
    /// **Note:** It is the responsibility of the user of this library to avoid calling
    /// this method multiple times. This will result in undefined radio behavior.
    ///
    /// # Arguments
    ///
    /// None
    ///
    /// # Returns
    ///
    /// A result indicating whether the transaction start packet was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// // Example 1: Use with `update_config` calls
    /// stream_api.start_config_transaction().await?;
    /// stream_api.update_config(packet_router, config_update_1).await?;
    /// stream_api.update_config(packet_router, config_update_2).await?;
    /// stream_api.commit_config_transaction().await?;
    ///
    /// // Example 2: Use with `set_local_config` call
    /// stream_api.start_config_transaction().await?;
    /// stream_api.set_local_config(packet_router, local_config).await?;
    /// stream_api.commit_config_transaction().await?;
    ///
    /// // Example 3: Updating module and channel configurations sequentially
    /// stream_api.start_config_transaction().await?;
    /// stream_api.update_module_config(packet_router, module_config).await?;
    /// stream_api.commit_config_transaction().await?;
    ///
    /// stream_api.start_config_transaction().await?;
    /// stream_api.update_channel_config(packet_router, channel_config).await?;
    /// stream_api.commit_config_transaction().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn start_config_transaction(&mut self) -> Result<(), String> {
        let to_radio = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::BeginEditSettings(
                true,
            )),
        };

        let mut packet_buf: Vec<u8> = vec![];
        to_radio
            .encode::<Vec<u8>>(&mut packet_buf)
            .map_err(|e| e.to_string())?;

        self.send_raw(packet_buf).await?;

        Ok(())
    }

    /// A method to tell the radio to complete a bulk configuration update.
    ///
    /// This method is intended to be used to batch multiple configuration updates into a single
    /// transaction on the radio. This is meant to avoid needing to wait for the radio
    /// to restart multiple times when updating multiple configuration fields.
    ///
    /// After calling this method, any calls to `update_config`, `update_module_config`, or `update_channel_config`
    /// will be buffered on the radio until the `commit_config_transaction` method is called. This will
    /// then trigger a radio restart, and the buffered configuration updates will be applied.
    ///
    /// **Note:** The radio will restart when it receives this packet.
    ///
    /// **Note:** It is not supported to batch configuration, module configuration,
    /// and channel configuration updates together. These must be done in separate transactions.
    /// This is a limitation of the current firmware.
    ///
    /// **Note:** It is the responsibility of the user of this library to avoid calling
    /// this method multiple times, and to avoid calling this method without first caling the
    /// `start_config_transaction` method. This will result in undefined radio behavior.
    ///
    /// # Arguments
    ///
    /// None
    ///
    /// # Returns
    ///
    /// A result indicating whether the transaction commit packet was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// stream_api.start_config_transaction().await?;
    /// stream_api.update_config(packet_router, config_update_1).await?;
    /// stream_api.update_config(packet_router, config_update_2).await?;
    /// stream_api.commit_config_transaction().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///    
    pub async fn commit_config_transaction(&mut self) -> Result<(), String> {
        let to_radio = protobufs::AdminMessage {
            payload_variant: Some(
                protobufs::admin_message::PayloadVariant::CommitEditSettings(true),
            ),
        };

        let mut packet_buf: Vec<u8> = vec![];

        to_radio
            .encode::<Vec<u8>>(&mut packet_buf)
            .map_err(|e| e.to_string())?;

        self.send_raw(packet_buf).await?;

        Ok(())
    }

    /// A helper method to update multiple configuration fields at once.
    ///
    /// This method is intended to simplify the process of updating multiple configuration
    /// fields at once. This method will call the `update_config` method for each configuration
    /// field that is specified in the `LocalConfig` struct. This method is intended
    /// to be used with the `start_config_transaction` and `commit_config_transaction` methods.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `local_config` - An instance of the `LocalConfig` struct to update the radio with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the config was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// stream_api.start_config_transaction().await?;
    /// stream_api.set_local_config(packet_router, local_config).await?;
    /// stream_api.commit_config_transaction().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn set_local_config<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        local_config: protobufs::LocalConfig,
    ) -> Result<(), String> {
        if let Some(c) = local_config.bluetooth {
            self.update_config(
                packet_router,
                protobufs::Config {
                    payload_variant: Some(protobufs::config::PayloadVariant::Bluetooth(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_config.device {
            self.update_config(
                packet_router,
                protobufs::Config {
                    payload_variant: Some(protobufs::config::PayloadVariant::Device(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_config.display {
            self.update_config(
                packet_router,
                protobufs::Config {
                    payload_variant: Some(protobufs::config::PayloadVariant::Display(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_config.lora {
            self.update_config(
                packet_router,
                protobufs::Config {
                    payload_variant: Some(protobufs::config::PayloadVariant::Lora(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_config.network {
            self.update_config(
                packet_router,
                protobufs::Config {
                    payload_variant: Some(protobufs::config::PayloadVariant::Network(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_config.position {
            self.update_config(
                packet_router,
                protobufs::Config {
                    payload_variant: Some(protobufs::config::PayloadVariant::Position(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_config.power {
            self.update_config(
                packet_router,
                protobufs::Config {
                    payload_variant: Some(protobufs::config::PayloadVariant::Power(c)),
                },
            )
            .await?;
        }

        Ok(())
    }

    /// A helper method to update multiple module configuration fields at once.
    ///
    /// This method is intended to simplify the process of updating multiple module configuration
    /// fields at once. This method will call the `update_module_config` method for each configuration
    /// field that is specified in the `LocalModuleConfig` struct. This method is intended
    /// to be used with the `start_config_transaction` and `commit_config_transaction` methods.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `local_module_config` - An instance of the `LocalModuleConfig` struct to update the radio with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the module config was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// stream_api.start_config_transaction().await?;
    /// stream_api.set_local_module_config(packet_router, local_module_config).await?;
    /// stream_api.commit_config_transaction().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn set_local_module_config<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        local_module_config: protobufs::LocalModuleConfig,
    ) -> Result<(), String> {
        if let Some(c) = local_module_config.audio {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(protobufs::module_config::PayloadVariant::Audio(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_module_config.canned_message {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(protobufs::module_config::PayloadVariant::CannedMessage(
                        c,
                    )),
                },
            )
            .await?;
        }

        if let Some(c) = local_module_config.external_notification {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(
                        protobufs::module_config::PayloadVariant::ExternalNotification(c),
                    ),
                },
            )
            .await?;
        }

        if let Some(c) = local_module_config.mqtt {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(protobufs::module_config::PayloadVariant::Mqtt(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_module_config.range_test {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(protobufs::module_config::PayloadVariant::RangeTest(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_module_config.remote_hardware {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(
                        protobufs::module_config::PayloadVariant::RemoteHardware(c),
                    ),
                },
            )
            .await?;
        }

        if let Some(c) = local_module_config.serial {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(protobufs::module_config::PayloadVariant::Serial(c)),
                },
            )
            .await?;
        }

        if let Some(c) = local_module_config.store_forward {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(protobufs::module_config::PayloadVariant::StoreForward(
                        c,
                    )),
                },
            )
            .await?;
        }

        if let Some(c) = local_module_config.telemetry {
            self.update_module_config(
                packet_router,
                protobufs::ModuleConfig {
                    payload_variant: Some(protobufs::module_config::PayloadVariant::Telemetry(c)),
                },
            )
            .await?;
        }

        Ok(())
    }

    /// A helper method to update the configuration of multiple message channels at once.
    ///
    /// This method is intended to simplify the process of updating multiple channel configuration
    /// fields at once. This method will call the `update_channel_config` method for each configuration
    /// field that is specified in the list of `Channel` structs. This method is intended
    /// to be used with the `start_config_transaction` and `commit_config_transaction` methods.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// This router is used in the event a packet needs to be echoed.
    /// * `channel_config` - A list of updates to make to radio channels.
    ///
    /// # Returns
    ///
    /// A result indicating whether the channel configuration updates were successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// stream_api.start_config_transaction().await?;
    /// stream_api.set_message_channel_config(packet_router, vec![ ... ]).await?;
    /// stream_api.commit_config_transaction().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the packet fails to send.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn set_message_channel_config<M, E: Display, R: PacketRouter<M, E>>(
        &mut self,
        packet_router: &mut R,
        channel_config: Vec<protobufs::Channel>,
    ) -> Result<(), String> {
        for channel in channel_config {
            self.update_channel_config(packet_router, channel).await?;
        }

        Ok(())
    }
}
