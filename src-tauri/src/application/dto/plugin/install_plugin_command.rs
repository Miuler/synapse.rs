use std::path::PathBuf;

pub struct InstallPluginCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub plugin_id: String,
    pub repo: String,
}
