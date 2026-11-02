use crate::application::dto::plugin::get_installed_plugins_query::GetInstalledPluginsQuery;
use crate::domain::plugin::installed_plugin_repository::InstalledPluginRepository;
use crate::domain::plugin::community_plugin::InstalledPluginInfo;

/// Plugins comunitarios instalados en una bóveda.
#[derive(Clone)]
pub struct GetInstalledPluginsUseCase<A: InstalledPluginRepository> {
    installed: A,
}

impl<A: InstalledPluginRepository> GetInstalledPluginsUseCase<A> {
    pub fn new(installed: A) -> Self {
        Self { installed }
    }

    pub fn execute(&self, query: GetInstalledPluginsQuery) -> Result<Vec<InstalledPluginInfo>, String> {
        self.installed.list_installed(&query.vault_path)
    }
}
