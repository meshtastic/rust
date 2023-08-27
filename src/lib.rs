pub mod connections;
pub mod utils;

#[cfg(feature = "ts-gen")]
pub mod ts {
    pub use specta::export::ts_with_cfg;
    pub use specta::ts::{
        BigIntExportBehavior, ExportConfiguration, ModuleExportBehavior, TsExportError,
    };
    pub use specta::Type;
}

pub use prost::Message;

pub mod protobufs {

    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/meshtastic.rs"));
}
