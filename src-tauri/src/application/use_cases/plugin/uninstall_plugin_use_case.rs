use crate::application::dto::plugin::uninstall_plugin_command::UninstallPluginCommand;
use crate::application::service::community_plugin_downloader::CommunityPluginDownloader;

/// Desinstala un plugin comunitario.
#[derive(Clone)]
pub struct UninstallPluginUseCase<A: CommunityPluginDownloader> {
    downloader: A,
}

impl<A: CommunityPluginDownloader> UninstallPluginUseCase<A> {
    pub fn new(downloader: A) -> Self {
        Self { downloader }
    }

    pub async fn execute(&self, command: UninstallPluginCommand) -> Result<(), String> {
        let clean_id = command.plugin_id.trim();
        if clean_id.is_empty() {
            return Err("El ID del plugin no puede estar vacío".to_string());
        }
        self.downloader.uninstall_plugin(&command.vault_path, clean_id).await
    }
}
