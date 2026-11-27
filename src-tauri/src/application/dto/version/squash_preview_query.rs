use std::path::PathBuf;

pub struct SquashPreviewQuery {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    /// Primer commit que se uniría.
    pub from: String,
}
