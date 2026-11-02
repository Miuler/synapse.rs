use std::path::PathBuf;

pub struct ShowItemInFolderCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub relative_path: String,
}
