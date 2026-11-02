use std::path::PathBuf;

pub struct CopyVaultItemsCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub paths: Vec<String>,
    pub dest_dir: String,
}
