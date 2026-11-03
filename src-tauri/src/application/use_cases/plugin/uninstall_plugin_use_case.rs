use crate::application::dto::plugin::uninstall_plugin_command::UninstallPluginCommand;
use crate::application::service::community_plugin_downloader::CommunityPluginDownloader;

/// Desinstala un plugin comunitario.
#[derive(Clone)]
pub struct UninstallPluginUseCase<A: CommunityPluginDownloader> {
    community_plugin_downloader: A,
}

impl<A: CommunityPluginDownloader> UninstallPluginUseCase<A> {
    pub fn new(community_plugin_downloader: A) -> Self {
        Self {
            community_plugin_downloader,
        }
    }

    pub async fn execute(&self, command: UninstallPluginCommand) -> Result<(), String> {
        let clean_id = command.plugin_id.trim();
        if clean_id.is_empty() {
            return Err("El ID del plugin no puede estar vacío".to_string());
        }
        self.community_plugin_downloader
            .uninstall_plugin(&command.vault_path, clean_id)
            .await
    }
}
