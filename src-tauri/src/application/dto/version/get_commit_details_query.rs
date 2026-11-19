use std::path::PathBuf;

pub struct GetCommitDetailsQuery {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    pub commit: String,
}
