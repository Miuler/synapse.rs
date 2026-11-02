use crate::application::dto::plugin::check_plugin_updates_query::CheckPluginUpdatesQuery;
use crate::application::service::community_plugin_downloader::CommunityPluginDownloader;
use crate::domain::model::community_plugin::PluginUpdateInfo;

/// Comprueba si hay versiones nuevas de plugins instalados.
#[derive(Clone)]
pub struct CheckPluginUpdatesUseCase<A: CommunityPluginDownloader> {
    downloader: A,
}

impl<A: CommunityPluginDownloader> CheckPluginUpdatesUseCase<A> {
    pub fn new(downloader: A) -> Self {
        Self { downloader }
    }

    pub async fn execute(&self, query: CheckPluginUpdatesQuery) -> Result<Vec<PluginUpdateInfo>, String> {
        self.downloader.check_plugin_updates(&query.plugins).await
    }
}
