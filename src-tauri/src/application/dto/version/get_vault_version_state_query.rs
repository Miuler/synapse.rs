use std::path::PathBuf;

pub struct GetVaultVersionStateQuery {
    /// Bóveda activa.
    pub vault_path: PathBuf,
}
