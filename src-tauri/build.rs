use anyhow::Result;
use vergen_gix::{BuildBuilder, Emitter, GixBuilder};

fn main() -> Result<()> {
    // Timestamp de compilación (UTC)
    let build = BuildBuilder::default().build_timestamp(true).build()?;

    // `git describe --tags --dirty` y estado sucio del árbol de trabajo
    let gix = GixBuilder::default()
        .describe(true, true, None)
        .dirty(true)
        .build()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .emit()?;

    tauri_build::build();
    Ok(())
}
