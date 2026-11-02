use std::path::PathBuf;

pub struct GetFileRevisionQuery {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub path: String,
    pub staged: bool,
    pub revision: Option<String>,
}
