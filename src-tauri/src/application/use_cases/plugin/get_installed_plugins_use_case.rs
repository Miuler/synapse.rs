use crate::application::dto::plugin::get_installed_plugins_query::GetInstalledPluginsQuery;
use crate::domain::plugin::community_plugin::InstalledPluginInfo;
use crate::domain::plugin::installed_plugin_repository::InstalledPluginRepository;

/// Plugins comunitarios instalados en una bóveda.
#[derive(Clone)]
pub struct GetInstalledPluginsUseCase<A: InstalledPluginRepository> {
    installed_plugin_repository: A,
}

impl<A: InstalledPluginRepository> GetInstalledPluginsUseCase<A> {
    pub fn new(installed_plugin_repository: A) -> Self {
        Self {
            installed_plugin_repository,
        }
    }

    pub fn execute(
        &self,
        query: GetInstalledPluginsQuery,
    ) -> Result<Vec<InstalledPluginInfo>, String> {
        self.installed_plugin_repository
            .list_installed(&query.vault_path)
    }
}
