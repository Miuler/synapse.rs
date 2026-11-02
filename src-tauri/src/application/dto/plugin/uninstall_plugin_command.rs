use std::path::PathBuf;

pub struct UninstallPluginCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub plugin_id: String,
}
