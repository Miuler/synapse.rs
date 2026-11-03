use crate::application::dto::plugin::fetch_community_plugins_query::FetchCommunityPluginsQuery;
use crate::application::service::community_plugin_downloader::CommunityPluginDownloader;
use crate::domain::plugin::community_plugin::CommunityPlugin;

/// Catálogo de plugins comunitarios.
#[derive(Clone)]
pub struct FetchCommunityPluginsUseCase<A: CommunityPluginDownloader> {
    community_plugin_downloader: A,
}

impl<A: CommunityPluginDownloader> FetchCommunityPluginsUseCase<A> {
    pub fn new(community_plugin_downloader: A) -> Self {
        Self {
            community_plugin_downloader,
        }
    }

    pub async fn execute(
        &self,
        query: FetchCommunityPluginsQuery,
    ) -> Result<Vec<CommunityPlugin>, String> {
        self.community_plugin_downloader
            .fetch_community_plugins(query.force_refresh)
            .await
    }
}
