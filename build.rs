use walkdir::WalkDir;

fn main() -> std::io::Result<()> {
    let protobufs_dir = "src/protobufs/";
    println!("cargo:rerun-if-changed={}", protobufs_dir);

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

    let mut protos = vec![];

    for entry in WalkDir::new(protobufs_dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext.to_str().unwrap() == "proto")
        })
    {
        let path = entry.path();
        protos.push(path.to_owned());
    }

    let mut config = prost_build::Config::new();

    let mut derive_string = String::from("#[derive(");

    #[cfg(feature = "serde")]
    {
        derive_string.push_str("serde::Serialize, serde::Deserialize, ");
    }

    #[cfg(feature = "ts-gen")]
    {
        derive_string.push_str("specta::Type, ");
    }

    derive_string.push_str(")]");

    config.type_attribute(".", derive_string.as_str());

    #[cfg(feature = "serde")]
    {
        config.type_attribute(".", "#[serde(rename_all = \"camelCase\")]");
        config.type_attribute(".", "#[allow(clippy::doc_lazy_continuation)]");
    }

    config.compile_protos(&protos, &[protobufs_dir]).unwrap();

    Ok(())
}
