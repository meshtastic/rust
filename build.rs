//! The `build.rs` script for this crate.

#[cfg(not(feature = "gen"))]
fn main() {}

#[cfg(feature = "gen")]
fn main() -> std::io::Result<()> {
    let src_dir = "src/protobufs/";
    let gen_dir = "src/generated/";

    println!("cargo:rerun-if-changed={src_dir}");
    println!("cargo:rerun-if-changed={gen_dir}");

    // Allows protobuf compilation without installing the `protoc` binary
    match protoc_bin_vendored::protoc_bin_path() {
        Ok(protoc_path) => {
            if std::env::var("PROTOC").ok().is_some() {
                println!("Using PROTOC set in environment.");
            } else {
                println!("Setting PROTOC to protoc-bin-vendored version.");
                std::env::set_var("PROTOC", protoc_path);
            }
        }
        Err(err) => {
            println!("Install protoc yourself, protoc-bin-vendored failed: {err}");
        }
    }
    // Get sorted list of .proto files inside src_dir
    let mut protos: Vec<_> = walkdir::WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|x| x.ok())
        .map(|x| x.into_path())
        .filter(|x| x.extension().is_some_and(|x| x == "proto"))
        .collect();
    protos.sort();

    let mut config = prost_build::Config::new();
    config.type_attribute(
        ".",
        "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]",
    );
    config.type_attribute(
        ".",
        "#[cfg_attr(feature = \"serde\", serde(rename_all = \"camelCase\"))]",
    );
    config.type_attribute(
        ".",
        "#[cfg_attr(feature = \"ts-gen\", derive(specta::Type))]",
    );

    config.out_dir(gen_dir);
    config.compile_protos(&protos, &[src_dir])
}
