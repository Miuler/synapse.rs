use std::path::PathBuf;

pub struct GetRepoLogQuery {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    pub max_count: Option<usize>,
}
