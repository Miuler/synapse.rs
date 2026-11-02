use crate::domain::plugin::community_plugin::InstalledPluginInfo;
use std::path::Path;

/// Plugins comunitarios instalados en la bóveda.
pub trait InstalledPluginRepository: Send + Sync {
    fn list_installed(&self, vault_path: &Path) -> Result<Vec<InstalledPluginInfo>, String>;
}
