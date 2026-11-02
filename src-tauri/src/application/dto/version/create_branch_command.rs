use std::path::PathBuf;

pub struct CreateBranchCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub new_branch: String,
    pub base_branch: String,
    pub checkout: bool,
}
