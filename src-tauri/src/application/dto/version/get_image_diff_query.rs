use std::path::PathBuf;

pub struct GetImageDiffQuery {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub path: String,
    pub staged: bool,
    pub revision: Option<String>,
}
