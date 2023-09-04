pub mod connections;
pub mod errors;
pub mod utils;

#[cfg(feature = "ts-gen")]
pub mod ts {
    #![allow(non_snake_case)]
    pub use specta;
}

pub use prost::Message;

pub type PacketReceiver = tokio::sync::mpsc::UnboundedReceiver<protobufs::FromRadio>;

pub mod protobufs {

    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/meshtastic.rs"));
}
