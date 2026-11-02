use std::path::PathBuf;

pub struct CheckoutBranchCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub branch_name: String,
}
