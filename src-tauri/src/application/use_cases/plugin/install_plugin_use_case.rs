use crate::application::dto::plugin::install_plugin_command::InstallPluginCommand;
use crate::application::service::community_plugin_downloader::CommunityPluginDownloader;
use crate::domain::plugin::community_plugin::CommunityPluginInstallResult;

/// Instala un plugin comunitario validando su id y su repositorio.
#[derive(Clone)]
pub struct InstallPluginUseCase<A: CommunityPluginDownloader> {
    community_plugin_downloader: A,
}

impl<A: CommunityPluginDownloader> InstallPluginUseCase<A> {
    pub fn new(community_plugin_downloader: A) -> Self {
        Self {
            community_plugin_downloader,
        }
    }

    pub async fn execute(
        &self,
        command: InstallPluginCommand,
    ) -> Result<CommunityPluginInstallResult, String> {
        let clean_id = command.plugin_id.trim();
        let clean_repo = command.repo.trim();

        if clean_id.is_empty() {
            return Err("El ID del plugin no puede estar vacío".to_string());
        }
        if clean_repo.is_empty() || !clean_repo.contains('/') {
            return Err(format!(
                "Repositorio inválido: '{clean_repo}'. Debe tener formato 'usuario/repo'"
            ));
        }

        self.community_plugin_downloader
            .install_plugin(&command.vault_path, clean_id, clean_repo)
            .await
    }
}
