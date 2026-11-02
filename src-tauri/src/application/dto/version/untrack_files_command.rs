use std::path::PathBuf;

pub struct UntrackFilesCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub paths: Vec<String>,
}
