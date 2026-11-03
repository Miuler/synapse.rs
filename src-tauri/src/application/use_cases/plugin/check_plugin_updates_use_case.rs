use crate::application::dto::plugin::check_plugin_updates_query::CheckPluginUpdatesQuery;
use crate::application::service::community_plugin_downloader::CommunityPluginDownloader;
use crate::domain::plugin::community_plugin::PluginUpdateInfo;

/// Comprueba si hay versiones nuevas de plugins instalados.
#[derive(Clone)]
pub struct CheckPluginUpdatesUseCase<A: CommunityPluginDownloader> {
    community_plugin_downloader: A,
}

impl<A: CommunityPluginDownloader> CheckPluginUpdatesUseCase<A> {
    pub fn new(community_plugin_downloader: A) -> Self {
        Self {
            community_plugin_downloader,
        }
    }

    pub async fn execute(
        &self,
        query: CheckPluginUpdatesQuery,
    ) -> Result<Vec<PluginUpdateInfo>, String> {
        self.community_plugin_downloader
            .check_plugin_updates(&query.plugins)
            .await
    }
}
