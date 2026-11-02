use std::path::PathBuf;

pub struct DeleteVaultItemCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub relative_path: String,
}
