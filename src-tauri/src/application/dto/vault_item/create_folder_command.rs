use std::path::PathBuf;

pub struct CreateFolderCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub relative_path: String,
}
