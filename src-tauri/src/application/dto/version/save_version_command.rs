use std::path::PathBuf;

pub struct SaveVersionCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub paths: Vec<String>,
    pub message: String,
}
