use std::path::PathBuf;

pub struct WriteBinaryVaultItemCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub bytes: Vec<u8>,
}
