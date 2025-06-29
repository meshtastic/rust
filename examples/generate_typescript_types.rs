//! This example connects to a radio via serial, and demonstrates how to
//! configure handlers for different types of decoded radio packets.
//! https://meshtastic.org/docs/supported-hardware
//!
//! Run this example with the command `cargo run --example generate_typescript_types --features "ts-gen"`
extern crate meshtastic;

use meshtastic::ts::specta::{
    export::ts_with_cfg,
    ts::{BigIntExportBehavior, ExportConfiguration, ModuleExportBehavior, TsExportError},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Exports relative to the root workspace directory
    export_ts_types("./examples/bindings.ts")?;

    Ok(())
}

fn export_ts_types(file_path: &str) -> Result<(), TsExportError> {
    // Sets up a default configuration for exporting typescript types
    let ts_export_config = ExportConfiguration::default()
        .bigint(BigIntExportBehavior::String)
        .modules(ModuleExportBehavior::Enabled);

    // Use Specta helper function to export typescript types
    ts_with_cfg(file_path, &ts_export_config)
}
