//! This example exports meshtastic/protobufs Rust types into TypeScript
//!
//! Run this example with the command `cargo run --example generate_typescript_types --features "ts-gen"`
extern crate meshtastic;

use specta_typescript::Typescript;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Exports relative to the root workspace directory
    Typescript::default()
        .bigint(specta_typescript::BigIntExportBehavior::String)
        .format(specta_typescript::Format::ModulePrefixedName)
        .export_to("./examples/bindings.ts", &specta::export())?;

    Ok(())
}
