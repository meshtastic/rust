use std::fmt::Display;

use crate::protobufs;

use self::wrappers::NodeId;

#[cfg(feature = "bluetooth-le")]
pub mod ble_handler;
pub mod handlers;
pub mod stream_api;
pub mod stream_buffer;
pub mod wrappers;

/// An enum that defines the possible destinations for a mesh packet.
/// This enum is used to specify the destination of a packet when sending
/// a packet to the radio.
///
/// # Variants
///
/// * `Local` - A packet that should be handled by the connected node.
/// * `Broadcast` - A packet that should be broadcast to all nodes in the mesh.
/// * `Node(u32)` - A packet that should be sent to a specific node in the mesh,
///     specified by the passed `u32` id.
///
/// # Default
///
/// The default value for this enum is `Broadcast`.
#[derive(Clone, Copy, Debug, Default)]
pub enum PacketDestination {
    Local,
    #[default]
    Broadcast,
    Node(NodeId),
}

/// This trait defines the behavior of a struct that is able to route mesh packets.
/// More generally, this trait defines the behavior of a struct that is able to send
/// and receive mesh packets.
///
/// The primary usage of this trait is to enable the management of packets within the
/// `send_packet` method. This method needs to be able to echo packets back to the client,
/// and is only able to do this if the `send_packet` method has the ability to trigger
/// the handling of arbitrary mesh packets.
pub trait PacketRouter<M: Sized, E: Display + std::error::Error + 'static> {
    /// A method that is used to handle `FromRadio` packets that are received from the radio.
    ///
    /// This method is generic on the `M` type, which allows the developer to return metadata on how the
    /// packet was handled. This metadata can then be used to trigger side effects unrelated to packet
    /// routing, such as updating a database.
    ///
    /// This method is also generic on the `E` type, which allows the developer to specify an error return type.
    ///
    /// # Arguments
    ///
    /// * `packet` - A `FromRadio` packet to handle.
    ///
    /// # Returns
    ///
    /// A result resolving to the specified handling metadata generic type.
    ///
    /// # Examples
    ///
    /// ```
    /// use PacketRouter;
    ///
    /// struct HandlerMetadata {
    ///    should_update_db: bool,
    /// }
    ///
    /// let packet = protobufs::FromRadio { ... };
    /// let metadata = router.handle_packet_from_radio::<HandlerMetadata, String>(packet).unwrap();
    ///
    /// println!("Should update db: {}", metadata.should_update_db);
    ///
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if packet handling fails for any reason.
    ///
    /// # Panics
    ///
    /// None
    ///
    fn handle_packet_from_radio(&mut self, packet: protobufs::FromRadio) -> Result<M, E>;

    /// A method that is used to handle `MeshPacket` packets that are received from the radio.
    ///
    /// This method is generic on the `M` type, which allows the developer to return metadata on how the
    /// packet was handled. This metadata can then be used to trigger side effects unrelated to packet
    /// routing, such as updating a database.
    ///
    /// This method is also generic on the `E` type, which allows the developer to specify an error return type.
    ///
    /// # Arguments
    ///
    /// * `packet` - A `MeshPacket` packet to handle.
    ///
    /// # Returns
    ///
    /// A result resolving to the specified handling metadata generic type.
    ///
    /// # Examples
    ///
    /// ```
    /// use PacketRouter;
    ///
    /// struct HandlerMetadata {
    ///    should_update_db: bool,
    /// }
    ///
    /// let packet = protobufs::MeshPacket { ... };
    /// let metadata = router.handle_mesh_packet::<HandlerMetadata, String>(packet).unwrap();
    ///
    /// println!("Should update db: {}", metadata.should_update_db);
    ///
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if packet handling fails for any reason.
    ///
    /// # Panics
    ///
    /// None
    ///
    fn handle_mesh_packet(&mut self, packet: protobufs::MeshPacket) -> Result<M, E>;

    /// A method that allows the `send_packet` method to query the router for the current node id.
    ///
    /// This method will be used internally to specify the `from` field on outgoing mesh packets.
    /// This is used in mesh packet routing to allow nodes to selectively relay packets.
    ///
    /// **Note:** This must match the id of any connected device to ensure that configuration packets
    /// are received and handled correctly on the radio.
    ///
    /// # Arguments
    ///
    /// None
    ///
    /// # Returns
    ///
    /// The id to be used as the source node id for outgoing packets.
    ///
    /// # Examples
    ///
    /// ```
    /// let source_node_id = router.source_node_id();
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
    fn source_node_id(&self) -> NodeId;
}
