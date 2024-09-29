use futures_util::future::join3;
use log::trace;
use prost::Message;
use std::{fmt::Display, marker::PhantomData};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::UnboundedSender,
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{errors_internal::Error, protobufs, types::EncodedToRadioPacketWithHeader, utils};
use crate::{
    packet::PacketReceiver,
    utils_internal::{current_epoch_secs_u32, generate_rand_id},
};

use super::{
    handlers,
    wrappers::{
        encoded_data::{EncodedMeshPacketData, EncodedToRadioPacket, IncomingStreamData},
        mesh_channel::MeshChannel,
        NodeId,
    },
    PacketDestination, PacketRouter,
};

/// These structs are needed to guarantee that the `StreamApi` struct connection
/// methods are called in the correct order. This is done by using the typestate
/// pattern, which is a way of using the type system to enforce state transitions.
///
/// These structs are not intended to be used outside of the library.
///
/// Reference: <https://github.com/letsgetrusty/generics_and_zero_sized_types/blob/master/src/main.rs>
pub mod state {

    /// A unit struct indicating that the `ConnectedStreamApi` struct is in the `Connected` state.
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
    pub struct Connected;

    /// A unit struct indicating that the `ConnectedStreamApi` struct is in the `Configured` state.
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
    pub struct Configured;
}

// StreamApi definition

/// A struct that provides a high-level API for communicating with a Meshtastic radio.
///
/// The `StreamApi` struct starts in a disconnected state, and the user must call
/// the `connect` command before being able to send and receive data from the radio.
/// This will return an instance of the `ConnectedStreamApi` struct, which then allows the
/// developer to call the `configure` method. The developer will then be able to interact with
/// the radio by calling the various "send" methods, which will send packets onto the mesh.
#[derive(Debug)]
pub struct StreamApi;

/// A struct that provides a high-level API for communicating with a Meshtastic radio.
///
/// This struct cannot be created directly, and must be created by calling the `connect` method
/// on the `StreamApi` struct. Once the user has called `StreamApi::connect`, the user is then expected
/// to call the `configure` method on the resulting `ConnectedStreamApi` instance. The "send" methods
/// will not be available to the user until `configure` has been called, as the device will not
/// repond to them.
///
/// This struct can either be in the `Connected`, or `Configured` state. The `Connected` state is
/// used to indicate that the user has connected to a radio, but that the device connection has not
/// yet been configured. The `Configured` state is used to indicate that the `configure` method has been called
/// and that the device will respond to "send" methods.
#[derive(Debug)]
pub struct ConnectedStreamApi<State = state::Configured> {
    write_input_tx: UnboundedSender<EncodedToRadioPacketWithHeader>,

    read_handle: JoinHandle<Result<(), Error>>,
    write_handle: JoinHandle<Result<(), Error>>,
    processing_handle: JoinHandle<Result<(), Error>>,
    heartbeat_handle: JoinHandle<Result<(), Error>>,

    cancellation_token: CancellationToken,

    typestate: PhantomData<State>,
}

/// A struct that provides a reference to an underlying stream for reading/writing data and
/// potentially an accompanying join handle that processes data on the other side of the stream.
pub struct StreamHandle<T: AsyncReadExt + AsyncWriteExt + Send> {
    pub stream: T,
    pub join_handle: Option<JoinHandle<Result<(), Error>>>,
}

impl<T: AsyncReadExt + AsyncWriteExt + Send> StreamHandle<T> {
    pub fn from_stream(stream: T) -> Self {
        Self {
            stream,
            join_handle: None,
        }
    }
}

// Packet helper functions

impl<State> ConnectedStreamApi<State> {
    /// A helper method to send encoded byte data to the radio within a MeshPacket wrapper.
    /// This method is generally intended for advanced users and should only be used when the
    /// more specific "send" methods are not sufficient.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    /// * `byte_data` - A `Vec<u8>` containing the byte data to send.
    /// * `port_num` - A `PortNum` enum that specifies the port number to send the packet on.
    /// * `destination` - A `PacketDestination` enum that specifies the destination of the packet.
    /// * `channel` - A `u32` that specifies the message channel to send the packet on, in the range [0..7).
    /// * `want_ack` - A `bool` that specifies whether or not the radio should wait for acknowledgement
    ///     from other nodes on the mesh.
    /// * `want_response` - A `bool` that specifies whether or not the radio should wait for a response
    ///     from other nodes on the mesh.
    /// * `echo_response` - A `bool` that specifies whether or not the radio should echo the packet back
    ///     to the client.
    /// * `reply_id` - An optional `u32` that specifies the ID of the packet to reply to.
    /// * `emoji` - An optional `u32` that specifies the unicode emoji data to send with the packet.
    ///
    /// # Returns
    ///
    /// A result indicating whether the packet was successfully dispatched to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// // Example 1: Send a text message to a node
    /// let byte_data = "Hello, world!".to_string().into_bytes();
    ///
    /// self.send_mesh_packet(
    ///     packet_router,
    ///     byte_data,
    ///     protobufs::PortNum::TextMessageApp,
    ///     destination,
    ///     channel,
    ///     want_ack,
    ///     false,
    ///     true,
    ///     None,
    ///     None,
    /// )
    /// .await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Return an error based on whether the packet is successfully dispatched to the radio.
    ///
    /// # Panics
    ///
    /// None
    ///
    #[allow(clippy::too_many_arguments)]
    pub async fn send_mesh_packet<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        packet_data: EncodedMeshPacketData,
        port_num: protobufs::PortNum,
        destination: PacketDestination,
        channel: MeshChannel,
        want_ack: bool,
        want_response: bool,
        echo_response: bool,
        reply_id: Option<u32>,
        emoji: Option<u32>,
    ) -> Result<(), Error> {
        let own_node_id = packet_router.source_node_id();

        let packet_destination: NodeId = match destination {
            PacketDestination::Local => own_node_id,
            PacketDestination::Broadcast => u32::MAX.into(),
            PacketDestination::Node(id) => id,
        };

        // NOTE(canardleteer): We don't warn on deprecation here, because it
        //                     remains valid for many active nodes, and
        //                     remains a part of the generated interface.
        #[allow(deprecated)]
        let mut mesh_packet = protobufs::MeshPacket {
            payload_variant: Some(protobufs::mesh_packet::PayloadVariant::Decoded(
                protobufs::Data {
                    portnum: port_num as i32,
                    payload: packet_data.data_vec(),
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
            delayed: 0,   // * not transmitted [deprecated since protobufs v2.2.19]
            hop_start: 0, // * set on device
            via_mqtt: false,
            from: own_node_id.id(),
            to: packet_destination.id(),
            id: generate_rand_id(),
            want_ack,
            channel: channel.channel(),
        };

        if echo_response {
            mesh_packet.rx_time = current_epoch_secs_u32();
            packet_router
                .handle_mesh_packet(mesh_packet.clone())
                .map_err(|e| Error::PacketHandlerFailure {
                    source: Box::new(e),
                })?;
        }

        let payload_variant = Some(protobufs::to_radio::PayloadVariant::Packet(mesh_packet));
        self.send_to_radio_packet(payload_variant).await?;

        Ok(())
    }

    /// A helper method to send a raw `ToRadio` packet to the radio based on a provided `protobufs::to_radio::PayloadVariant`.
    /// This method is generally intended for advanced users and should only be used when the
    /// more specific "send" methods are not sufficient.
    ///
    /// # Arguments
    ///
    /// * `payload_variant` - An optional `PayloadVariant` enum that specifies the payload to attach to the `ToRadio` packet.
    ///
    /// # Returns
    ///
    /// A result indicating whether the packet was successfully dispatched to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// // Example 1: Send a mesh packet onto the mesh
    /// let mesh_packet = protobufs::MeshPacket { ... };
    /// let payload_variant = Some(protobufs::to_radio::PayloadVariant::Packet(mesh_packet));
    /// self.send_to_radio_packet(payload_variant).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error based on whether the packet is successfully encoded and dispatched to the radio.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn send_to_radio_packet(
        &mut self,
        payload_variant: Option<protobufs::to_radio::PayloadVariant>,
    ) -> Result<(), Error> {
        let packet = protobufs::ToRadio { payload_variant };

        let mut packet_buf: Vec<u8> = vec![];
        packet.encode::<Vec<u8>>(&mut packet_buf)?;
        self.send_raw(packet_buf.into()).await?;

        Ok(())
    }

    /// A helper method to send a raw `ToRadio` packet to the radio based on an encoded `ToRadio` packet.
    /// This method is generally intended for advanced users and should only be used when the
    /// more specific "send" methods are not sufficient.
    ///
    /// # Arguments
    ///
    /// * `packet_buf` - A `Vec<u8>` containing the encoded `ToRadio` packet to send.
    ///
    /// # Returns
    ///
    /// A result indicating whether the packet was successfully dispatched to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// // Example 1: Encode and send a `ToRadio` packet based on variant
    /// let packet = protobufs::ToRadio { payload_variant };
    ///
    /// let mut packet_buf: Vec<u8> = vec![];
    /// packet.encode::<Vec<u8>>(&mut packet_buf)?;
    /// self.send_raw(packet_buf).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error based on whether the packet is successfully encoded and dispatched to the radio.
    /// This method will fail if the channel fails to send the encoded packet.
    ///
    /// # Panics
    ///
    /// None
    ///
    pub async fn send_raw(&mut self, data: EncodedToRadioPacket) -> Result<(), Error> {
        let channel = self.write_input_tx.clone();
        let data_with_header = utils::format_data_packet(data)?;

        channel
            .send(data_with_header)
            .map_err(|e| Error::InternalChannelError(e.into()))?;

        Ok(())
    }

    /// A helper method to allow advanced users access to the internal `UnboundedSender` channel
    /// used to send raw data to the radio. This method is generally intended for advanced users
    /// and should only be used when the more specific "send" methods are not sufficient. This
    /// method returns a copy of the internal `tokio::sync::mpsc::UnboundedSender` sender channel.
    ///
    /// This method is intended to be used when a user needs very low-level access to the radio
    /// interface, for example to send a packet that isn't supported by the current "send" methods.
    ///
    /// **Note:** This sender is only intended to be used to send encoded packet data. This **does not**
    /// include the 4 packet header bytes, which are attached to the payload in an internal worker thread.
    ///
    /// # Arguments
    ///
    /// None
    ///
    /// # Returns
    ///
    /// Returns an `UnboundedSender` channel that can be used to send raw data to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let write_input_sender = stream_api.write_input_sender();
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
    pub fn write_input_sender(&self) -> UnboundedSender<EncodedToRadioPacketWithHeader> {
        self.write_input_tx.clone()
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
    #[allow(clippy::new_without_default)]
    pub fn new() -> StreamApi {
        StreamApi
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
    /// let stream_api = StreamApi::new();
    /// let serial_stream = build_serial_stream("/dev/ttyUSB0".to_string(), None, None, None)?;
    /// let (decoded_listener, stream_api) = stream_api.connect(serial_stream).await;
    ///
    /// // Example 2: Connect to a TCP port
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
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
    pub async fn connect<S>(
        self,
        stream_handle: StreamHandle<S>,
    ) -> (PacketReceiver, ConnectedStreamApi<state::Connected>)
    where
        S: AsyncReadExt + AsyncWriteExt + Send + 'static,
    {
        // Create message channels

        let (write_input_tx, write_input_rx) =
            tokio::sync::mpsc::unbounded_channel::<EncodedToRadioPacketWithHeader>();

        let (read_output_tx, read_output_rx) =
            tokio::sync::mpsc::unbounded_channel::<IncomingStreamData>();

        let (decoded_packet_tx, decoded_packet_rx) =
            tokio::sync::mpsc::unbounded_channel::<protobufs::FromRadio>();

        // Spawn worker threads with kill switch

        let (read_stream, write_stream) = tokio::io::split(stream_handle.stream);
        let cancellation_token = CancellationToken::new();

        let read_handle =
            handlers::spawn_read_handler(cancellation_token.clone(), read_stream, read_output_tx);

        let write_handle =
            handlers::spawn_write_handler(cancellation_token.clone(), write_stream, write_input_rx);

        let processing_handle = handlers::spawn_processing_handler(
            cancellation_token.clone(),
            read_output_rx,
            decoded_packet_tx,
        );

        let heartbeat_handle =
            handlers::spawn_heartbeat_handler(cancellation_token.clone(), write_input_tx.clone());

        // Persist channels and kill switch to struct

        let write_input_tx = write_input_tx;
        let cancellation_token = cancellation_token;

        // Return channel for receiving decoded packets

        (
            decoded_packet_rx,
            ConnectedStreamApi::<state::Connected> {
                write_input_tx,
                read_handle,
                write_handle,
                processing_handle,
                heartbeat_handle,
                cancellation_token,
                typestate: PhantomData,
            },
        )
    }
}

impl ConnectedStreamApi<state::Connected> {
    /// This method is used to trigger the transmission of the current state of the
    /// radio, as well as to subscribe to future `FromRadio` mesh packets. This method
    /// can only be called after the `connect` method has been called.
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
    ///     to check that the configuration process has completed.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating whether or not the configuration was successful.
    /// The configuration will fail if the `WantConfigId` packet fails to send.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
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
    pub async fn configure(
        mut self,
        config_id: u32,
    ) -> Result<ConnectedStreamApi<state::Configured>, Error> {
        let to_radio = protobufs::ToRadio {
            payload_variant: Some(protobufs::to_radio::PayloadVariant::WantConfigId(config_id)),
        };

        let packet_buf: EncodedToRadioPacket = to_radio.encode_to_vec().into();
        self.send_raw(packet_buf).await?;

        Ok(ConnectedStreamApi::<state::Configured> {
            write_input_tx: self.write_input_tx,
            read_handle: self.read_handle,
            write_handle: self.write_handle,
            processing_handle: self.processing_handle,
            heartbeat_handle: self.heartbeat_handle,
            cancellation_token: self.cancellation_token,
            typestate: PhantomData,
        })
    }
}

impl ConnectedStreamApi<state::Configured> {
    /// A method to disconnect from a radio. This method will close all channels and
    /// join all worker threads. If connected via serial or TCP, this will also trigger
    /// the radio to terminate its current connection.
    ///
    /// This method can only be called after the `configure` method has been called.
    ///
    /// # Arguments
    ///
    /// None
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating whether or not the disconnection was successful.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
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
    pub async fn disconnect(self) -> Result<StreamApi, Error> {
        // Tell worker threads to shut down
        self.cancellation_token.cancel();

        // Close writer channel, which will kill worker threads

        drop(self.write_input_tx);

        // Close worker threads

        let (read_result, write_result, processing_result) =
            join3(self.read_handle, self.write_handle, self.processing_handle).await;

        // Note: we only return the first error.
        read_result??;
        write_result??;
        processing_result??;

        trace!("Handlers fully disconnected");

        Ok(StreamApi)
    }
}

// Public node management API

impl ConnectedStreamApi<state::Configured> {
    /// Sends the specified text content over the mesh.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    ///     This router is used in the event a packet needs to be echoed.
    /// * `text` - A `String` containing the text to send.
    /// * `destination` - A `PacketDestination` enum that specifies the destination of the packet.
    /// * `want_ack` - A `bool` that specifies whether or not the radio should wait for acknowledgement
    ///     from other nodes on the mesh.
    /// * `channel` - A `u32` that specifies the message channel to send the packet on [0..7).
    ///
    /// # Returns
    ///
    /// A result indicating whether the packet was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (_decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
    ///
    /// let config_id = generate_rand_id();
    /// let mut stream_api = stream_api.configure(config_id).await?;
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
    pub async fn send_text<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        text: String,
        destination: PacketDestination,
        want_ack: bool,
        channel: MeshChannel,
    ) -> Result<(), Error> {
        let byte_data: EncodedMeshPacketData = text.into_bytes().into();

        self.send_mesh_packet(
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
    ///     This router is used in the event a packet needs to be echoed.
    /// * `waypoint` - An instance of the `Waypoint` struct to send.
    /// * `destination` - A `PacketDestination` enum that specifies the destination of the packet.
    /// * `want_ack` - A `bool` that specifies whether or not the radio should wait for acknowledgement
    ///     from other nodes on the mesh.
    /// * `channel` - A `u32` that specifies the message channel to send the packet on [0..7).
    ///
    /// # Returns
    ///
    /// A result indicating whether the packet was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (_decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
    ///
    /// let config_id = generate_rand_id();
    /// let mut stream_api = stream_api.configure(config_id).await?;
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
    pub async fn send_waypoint<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        waypoint: crate::protobufs::Waypoint,
        destination: PacketDestination,
        want_ack: bool,
        channel: MeshChannel,
    ) -> Result<(), Error> {
        let mut waypoint = waypoint;

        // Waypoint with ID of zero denotes a new waypoint; check whether to generate its ID on backend
        if waypoint.id == 0 {
            waypoint.id = generate_rand_id();
        }

        let byte_data: EncodedMeshPacketData = waypoint.encode_to_vec().into();

        self.send_mesh_packet(
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

    /// Sends the specified `Positon` over the mesh.
    ///
    /// Sending a `Position` packet will update the internal position of the connected radio
    /// in addition to sending the packet over the mesh.
    ///
    /// **Note:** If you want to send a point of interest (POI) over the mesh, use `send_waypoint`.
    ///
    /// # Arguments
    ///
    /// * `packet_router` - A generic packet router field that implements the `PacketRouter` trait.
    ///     This router is used in the event a packet needs to be echoed.
    /// * `position` - An instance of the `Position` struct to send.
    /// * `destination` - A `PacketDestination` enum that specifies the destination of the packet.
    /// * `want_ack` - A `bool` that specifies whether or not the radio should wait for acknowledgement
    ///     from other nodes on the mesh.
    /// * `channel` - A `u32` that specifies the message channel to send the packet on [0..7).
    ///
    /// # Returns
    ///
    /// A result indicating whether the packet was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (_decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
    ///
    /// let config_id = generate_rand_id();
    /// let mut stream_api = stream_api.configure(config_id).await?;
    ///
    /// let position = crate::protobufs::Position { ... };
    /// stream_api.send_position(packet_router, position, PacketDestination::Broadcast, true, 0).await?;
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
    pub async fn send_position<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        position: crate::protobufs::Position,
        destination: PacketDestination,
        want_ack: bool,
        channel: MeshChannel,
    ) -> Result<(), Error> {
        let byte_data: EncodedMeshPacketData = position.encode_to_vec().into();

        self.send_mesh_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::PositionApp,
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
    ///     This router is used in the event a packet needs to be echoed.
    /// * `config` - An instance of the `Config` struct to update the radio with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the config was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (_decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
    ///
    /// let config_id = generate_rand_id();
    /// let mut stream_api = stream_api.configure(config_id).await?;
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
    pub async fn update_config<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        config: protobufs::Config,
    ) -> Result<(), Error> {
        let config_packet = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::SetConfig(config)),
        };

        let byte_data: EncodedMeshPacketData = config_packet.encode_to_vec().into();

        self.send_mesh_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::AdminApp,
            PacketDestination::Local,
            MeshChannel::new(0)?,
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
    ///     This router is used in the event a packet needs to be echoed.
    /// * `module_config` - An instance of the `ModuleConfig` struct to update the radio with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the module config was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (_decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
    ///
    /// let config_id = generate_rand_id();
    /// let mut stream_api = stream_api.configure(config_id).await?;
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
    pub async fn update_module_config<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        module_config: protobufs::ModuleConfig,
    ) -> Result<(), Error> {
        let module_config_packet = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::SetModuleConfig(
                module_config,
            )),
        };

        let byte_data: EncodedMeshPacketData = module_config_packet.encode_to_vec().into();

        self.send_mesh_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::AdminApp,
            PacketDestination::Local,
            MeshChannel::new(0)?,
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
    ///     This router is used in the event a packet needs to be echoed.
    /// * `channel_config` - An instance of the `Channel` struct to update the radio with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the channel config was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (_decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
    ///
    /// let config_id = generate_rand_id();
    /// let mut stream_api = stream_api.configure(config_id).await?;
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
    pub async fn update_channel_config<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        channel_config: protobufs::Channel,
    ) -> Result<(), Error> {
        // Tell device to update channels

        let channel_packet = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::SetChannel(
                channel_config,
            )),
        };

        let byte_data: EncodedMeshPacketData = channel_packet.encode_to_vec().into();

        self.send_mesh_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::AdminApp,
            PacketDestination::Local,
            MeshChannel::new(0)?,
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
    ///     This router is used in the event a packet needs to be echoed.
    /// * `user` - An instance of the `User` struct to update the radio user with.
    ///
    /// # Returns
    ///
    /// A result indicating whether the new `User` information was successfully sent to the radio.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream_api = StreamApi::new();
    /// let tcp_stream = build_tcp_stream("localhost:4403".to_string()).await?;
    /// let (_decoded_listener, stream_api) = stream_api.connect(tcp_stream).await;
    ///
    /// let config_id = generate_rand_id();
    /// let mut stream_api = stream_api.configure(config_id).await?;
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
    pub async fn update_user<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        user: protobufs::User,
    ) -> Result<(), Error> {
        let user_packet = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::SetOwner(user)),
        };

        let byte_data: EncodedMeshPacketData = user_packet.encode_to_vec().into();

        self.send_mesh_packet(
            packet_router,
            byte_data,
            protobufs::PortNum::AdminApp,
            PacketDestination::Local,
            MeshChannel::new(0)?,
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
    pub async fn start_config_transaction(&mut self) -> Result<(), Error> {
        let to_radio = protobufs::AdminMessage {
            payload_variant: Some(protobufs::admin_message::PayloadVariant::BeginEditSettings(
                true,
            )),
        };

        let mut packet_buf: Vec<u8> = vec![];
        to_radio.encode::<Vec<u8>>(&mut packet_buf)?;
        self.send_raw(packet_buf.into()).await?;

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
    pub async fn commit_config_transaction(&mut self) -> Result<(), Error> {
        let to_radio = protobufs::AdminMessage {
            payload_variant: Some(
                protobufs::admin_message::PayloadVariant::CommitEditSettings(true),
            ),
        };

        let mut packet_buf: Vec<u8> = vec![];
        to_radio.encode::<Vec<u8>>(&mut packet_buf)?;
        self.send_raw(packet_buf.into()).await?;

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
    ///     This router is used in the event a packet needs to be echoed.
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
    pub async fn set_local_config<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        local_config: protobufs::LocalConfig,
    ) -> Result<(), Error> {
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
    ///     This router is used in the event a packet needs to be echoed.
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
    pub async fn set_local_module_config<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        local_module_config: protobufs::LocalModuleConfig,
    ) -> Result<(), Error> {
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
    ///     This router is used in the event a packet needs to be echoed.
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
    pub async fn set_message_channel_config<
        M,
        E: Display + std::error::Error + Send + Sync + 'static,
        R: PacketRouter<M, E>,
    >(
        &mut self,
        packet_router: &mut R,
        channel_config: Vec<protobufs::Channel>,
    ) -> Result<(), Error> {
        for channel in channel_config {
            self.update_channel_config(packet_router, channel).await?;
        }

        Ok(())
    }
}
