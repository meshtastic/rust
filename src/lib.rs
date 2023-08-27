pub mod connections;
pub mod utils;

pub mod protobufs {
    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/meshtastic.rs"));
}
