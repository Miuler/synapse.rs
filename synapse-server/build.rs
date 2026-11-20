//! Genera el código Rust del contrato gRPC (`synapse.sync.v1`) y su `FileDescriptorSet`.

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let descriptor_path = out_dir.join("sync_descriptor.bin");

    println!("cargo:rerun-if-changed=proto/synapse/sync/v1/sync.proto");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(descriptor_path)
        .compile_protos(&["proto/synapse/sync/v1/sync.proto"], &["proto"])?;

    Ok(())
}
