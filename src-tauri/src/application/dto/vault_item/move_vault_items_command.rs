use std::path::PathBuf;

pub struct MoveVaultItemsCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub paths: Vec<String>,
    pub dest_dir: String,
}
