use std::path::PathBuf;

pub struct GetInstalledPluginsQuery {
    /// Bóveda activa.
    pub vault_path: PathBuf,
}
