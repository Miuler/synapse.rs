use std::path::PathBuf;

pub struct GetFileVersionHistoryQuery {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub path: String,
    pub max_count: Option<usize>,
}
