use std::path::PathBuf;

pub struct RenameVaultItemCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub new_name: String,
}
