//! A Rust library for communicating with and configuring Meshtastic devices.
#[cfg(feature = "tokio")]
pub(crate) mod connections;
#[cfg(feature = "tokio")]
pub(crate) mod errors_internal;
#[cfg(feature = "tokio")]
pub(crate) mod utils_internal;

/// A re-export of the `prost::Message` trait, which is required to call the `encode`
/// and `decode` methods on protocol buffer messages.
pub use prost::Message;

/// This module contains the main API for interacting with Meshtastic devices.
/// This module exposes the `StreamApi` and `ConnectedStreamApi` structs, as well
/// as helper states within the `state` module.
///
/// The user will create a new instance of the API through calling the `StreamApi::new()`
/// method. This will return a new `StreamApi` instance. The only method that is exposed
/// on this struct is the `connect` method. This is a compile-time check to force the user
/// of the library to connect to a radio before attempting to send data onto the mesh.
///
/// If successful, the `connect` method will return a tuple of the `ConnectedStreamApi`
/// instance, as well as a `PacketReceiver`. The `PacketReceiver` is a tokio channel
/// that can be used to listen to incoming packets from the radio. Since the user is now
/// connected to the radio. the resulting `ConnectedStreamApi` instance will then be able
/// to access the `configure` method, as well as some additional low-level sender methods.
///
/// The `configure` method requests the current radio configuration, will return an updated
/// instance of the `ConnectedStreamApi` struct. This resulting instance will then have access
/// to the full set of API sender methods.
///
/// To disconnect from the radio, the user can call the `disconnect` method at any time.
#[cfg(feature = "tokio")]
pub mod api {
    pub use crate::connections::stream_api::state;
    pub use crate::connections::stream_api::ConnectedStreamApi;
    pub use crate::connections::stream_api::StreamApi;
    pub use crate::connections::stream_api::StreamHandle;
}

/// This module contains the global `Error` type of the library. This enum implements
/// `std::error::Error`, `std::fmt::Display`, and `std::fmt::Debug`. This enum is used to
/// represent all errors that can occur within the library.
#[cfg(feature = "tokio")]
pub mod errors {
    pub use crate::errors_internal::Error;
}

/// This module contains the structs, enums, and traits that are necessary to define the behavior
/// of packets within the library. This module exposes the `PacketDestination` enum, the `PacketRouter`
/// trait, and the `PacketReceiver` type.
///
/// The `PacketDestination` enum is used to define the destination of a packet. The enum defines three possible
/// destinations for packets sent to the radio by the library:
///
/// * `PacketDestination::Local` - This destination is used for packets that are intended to be processed locally
///   by the radio and not to be forwarded to other nodes. An example of this would be local configuration packets.
/// * `PacketDestination::Broadcast` - This destination is used for packets that are intended to be broadcast to all
///   nodes in the mesh. This is the default enum variant. Text messages are commonly broadcasted to the entire mesh.
/// * `PacketDestination::Node(u32)` - This destination is used for packets that are intended to be sent to a specific
///   node in the mesh. The `u32` value is the node id of the node that the packet should be sent to. This is commonly
///   used for direct text messages.
///
/// The `PacketRouter` trait defines the behavior of a struct that is able to route mesh packets. This trait is used
/// to allow for the echoing of mesh packets within the `send_mesh_packet` method of the `ConnectedStreamApi` struct.
///
/// The `PacketReceiver` type defines the type of the tokio channel that is used to receive decoded packets from the radio.
/// This is intended to simplify the complexity of the underlying channel type.
#[cfg(feature = "tokio")]
pub mod packet {
    pub use crate::connections::handlers::CLIENT_HEARTBEAT_INTERVAL;
    pub use crate::connections::PacketDestination;
    pub use crate::connections::PacketRouter;

    /// A type alias for the tokio channel that is used to receive decoded `protobufs::FromRadio` packets from the radio.
    pub type PacketReceiver = tokio::sync::mpsc::UnboundedReceiver<crate::protobufs::FromRadio>;
}

/// This module contains structs and enums that are generated from the protocol buffer (protobuf)
/// definitions of the `meshtastic/protobufs` Git submodule. These structs and enums
/// are not edited directly, but are instead generated at build time.
pub mod protobufs {
    #![allow(missing_docs)]
    #![allow(non_snake_case)]
    #![allow(unknown_lints)]
    #![allow(clippy::empty_docs)]
    #![allow(clippy::doc_lazy_continuation)]
    #![allow(clippy::doc_overindented_list_items)]
    include!("generated/meshtastic.rs");
}

/// This module re-exports the `specta` crate, which is used to generate TypeScript
/// type definitions from the protobuf definitions of the `meshtastic/protobufs` Git submodule.
/// This module is only compiled if the `ts-gen` feature is enabled.
///
/// The `specta` crate exposes functionality that allows users of the library to export a
/// TypeScript type definition file containing TypeScript types for all members of the
/// `protobufs` module. This allows for complete type safety when interfacing with a TypeScript
/// application.
#[cfg(feature = "ts-gen")]
pub mod ts {
    #![allow(non_snake_case)]

    /// A re-export of the `specta` crate, which is used to generate TypeScript type definitions
    /// from the protobuf definitions of the `meshtastic/protobufs` Git submodule.
    pub use specta;
}

/// This module exposes utility functions that aren't fundamental to the operation of the
/// library, but simplify the configuration and usage of member methods.
///
/// The `DEFAULT_DTR_PIN_STATE` and `DEFAULT_RTS_PIN_STATE` constants are used to define the
/// default pin states of the DTR and RTS pins of the serial connection. The `DEFAULT_SERIAL_BAUD`
/// constant is used to define the default baud rate of incoming serial connections created by the
/// `build_serial_stream` method.
///
/// Additionally, this module exposes helper methods that are used internally to format data packets.
/// These methods are intended for use by more advanced users.
///
/// The `stream` module contains helper methods that are used to build connection stream instances.
#[cfg(feature = "tokio")]
pub mod utils {
    pub use crate::utils_internal::DEFAULT_DTR_PIN_STATE;
    pub use crate::utils_internal::DEFAULT_RTS_PIN_STATE;
    pub use crate::utils_internal::DEFAULT_SERIAL_BAUD;

    pub use crate::utils_internal::current_epoch_secs_u32;
    pub use crate::utils_internal::format_data_packet;
    pub use crate::utils_internal::generate_rand_id;
    pub use crate::utils_internal::strip_data_packet_header;

    /// This module contains utility functions that are used to build the `Stream` instances
    /// that are used to connect to the radio. Since the `StreamApi::connect` method only
    /// requires that streams implement the `tokio::io::AsyncReadExt` and `tokio::io::AsyncWriteExt`
    /// methods, there are countless ways a user could theoretically connect to a radio.
    ///
    /// This module exposes the `build_serial_stream` and `build_tcp_stream` methods, which
    /// simplify the process of initializing a connection stream. The vast majority of users will
    /// only need to use these two methods to connect to a radio. The `available_serial_ports` method
    /// can also be used to list all available serial ports on the host machine.
    pub mod stream {
        #[cfg(feature = "bluetooth-le")]
        pub use crate::connections::ble_handler::BleId;
        #[cfg(feature = "bluetooth-le")]
        pub use crate::utils_internal::available_ble_devices;
        pub use crate::utils_internal::available_serial_ports;
        #[cfg(feature = "bluetooth-le")]
        pub use crate::utils_internal::build_ble_stream;
        pub use crate::utils_internal::build_serial_stream;
        pub use crate::utils_internal::build_tcp_stream;
    }
}

/// This module exposes wrappers around common types that are used throughout the library.
/// These wrappers are used to simplify the API of the library, and to provide additional
/// type safety.
///
/// The `NodeId` struct is a wrapper around a `u32` value that represents the ID of a node
/// in the mesh. This struct is used to provide additional type safety when specifying
/// node IDs.
///
/// The `MeshChannel` enum is a wrapper around a `u32` value that represents the channel
/// of the mesh. This struct is used to provide additional type safety when specifying
/// mesh channels, as it will only allow channels with indices between 0 and 7, inclusive.
///
/// The `EncodedMeshPacketData` struct is a wrapper around a `Vec<u8>` value that represents
/// the payload data of a mesh packet (e.g., a text message).
///
/// The `EncodedToRadioPacket` struct is a wrapper around a `Vec<u8>` value that represents
/// the payload data of a packet that is intended to be sent to the radio. This struct
/// **does not** represent the full packet that is sent to the radio, as it does not include
/// the required packet header.
///
/// The `EncodedToRadioPacketWithHeader` struct is a wrapper around a `Vec<u8>` value that
/// represents the payload data of a packet that is intended to be sent to the radio. This
/// struct includes the required packet header, and can be sent to the radio.
#[cfg(feature = "tokio")]
pub mod types {
    pub use crate::connections::wrappers::encoded_data::EncodedMeshPacketData;
    pub use crate::connections::wrappers::encoded_data::EncodedToRadioPacket;
    pub use crate::connections::wrappers::encoded_data::EncodedToRadioPacketWithHeader;
    pub use crate::connections::wrappers::encoded_data::IncomingStreamData;
    pub use crate::connections::wrappers::mesh_channel::MeshChannel;
    pub use crate::connections::wrappers::NodeId;
}
