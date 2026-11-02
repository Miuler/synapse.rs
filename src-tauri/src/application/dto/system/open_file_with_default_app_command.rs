use std::path::PathBuf;

pub struct OpenFileWithDefaultAppCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub relative_path: String,
}
