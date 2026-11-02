use crate::application::dto::plugin::fetch_community_plugins_query::FetchCommunityPluginsQuery;
use crate::application::service::community_plugin_downloader::CommunityPluginDownloader;
use crate::domain::plugin::community_plugin::CommunityPlugin;

/// Catálogo de plugins comunitarios.
#[derive(Clone)]
pub struct FetchCommunityPluginsUseCase<A: CommunityPluginDownloader> {
    downloader: A,
}

impl<A: CommunityPluginDownloader> FetchCommunityPluginsUseCase<A> {
    pub fn new(downloader: A) -> Self {
        Self { downloader }
    }

    pub async fn execute(&self, query: FetchCommunityPluginsQuery) -> Result<Vec<CommunityPlugin>, String> {
        self.downloader.fetch_community_plugins(query.force_refresh).await
    }
}
