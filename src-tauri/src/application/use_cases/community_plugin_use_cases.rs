use crate::domain::models::community_plugin::{CommunityPlugin, CommunityPluginInstallResult};
use crate::domain::services::community_plugin_downloader::CommunityPluginDownloader;
use std::path::Path;

/// Casos de uso de plugins comunitarios de Obsidian.
///
/// Orquesta la descarga, instalación, desinstalación y consulta de plugins
/// abstrayéndose del proveedor de red subyacente.
pub struct CommunityPluginUseCases<D: CommunityPluginDownloader> {
    downloader: D,
}

impl<D: CommunityPluginDownloader> CommunityPluginUseCases<D> {
    pub fn new(downloader: D) -> Self {
        Self { downloader }
    }

    pub async fn fetch_community_plugins(&self) -> Result<Vec<CommunityPlugin>, String> {
        self.downloader.fetch_community_plugins().await
    }

    pub async fn install_plugin(
        &self,
        vault_path: &Path,
        plugin_id: &str,
        repo: &str,
    ) -> Result<CommunityPluginInstallResult, String> {
        let clean_id = plugin_id.trim();
        let clean_repo = repo.trim();

        if clean_id.is_empty() {
            return Err("El ID del plugin no puede estar vacío".to_string());
        }
        if clean_repo.is_empty() || !clean_repo.contains('/') {
            return Err(format!("Repositorio inválido: '{clean_repo}'. Debe tener formato 'usuario/repo'"));
        }

        self.downloader.install_plugin(vault_path, clean_id, clean_repo).await
    }

    pub async fn uninstall_plugin(&self, vault_path: &Path, plugin_id: &str) -> Result<(), String> {
        let clean_id = plugin_id.trim();
        if clean_id.is_empty() {
            return Err("El ID del plugin no puede estar vacío".to_string());
        }
        self.downloader.uninstall_plugin(vault_path, clean_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct MockPluginDownloader {
        installed: Arc<AtomicBool>,
        uninstalled: Arc<AtomicBool>,
    }

    impl CommunityPluginDownloader for MockPluginDownloader {
        fn fetch_community_plugins(
            &self,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<CommunityPlugin>, String>> + Send + '_>> {
            Box::pin(async {
                Ok(vec![CommunityPlugin {
                    id: "test-plugin".to_string(),
                    name: "Test Plugin".to_string(),
                    author: "Tester".to_string(),
                    description: "A test plugin".to_string(),
                    repo: "tester/test-plugin".to_string(),
                    branch: None,
                    downloads: Some(100),
                }])
            })
        }

        fn install_plugin<'a>(
            &'a self,
            _vault_path: &'a Path,
            plugin_id: &'a str,
            _repo: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<CommunityPluginInstallResult, String>> + Send + 'a>> {
            let installed = Arc::clone(&self.installed);
            Box::pin(async move {
                installed.store(true, Ordering::SeqCst);
                Ok(CommunityPluginInstallResult {
                    id: plugin_id.to_string(),
                    version: "1.0.0".to_string(),
                    installed_path: format!("/vault/.synapse/plugins/{plugin_id}"),
                    has_styles: true,
                })
            })
        }

        fn uninstall_plugin<'a>(
            &'a self,
            _vault_path: &'a Path,
            _plugin_id: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            let uninstalled = Arc::clone(&self.uninstalled);
            Box::pin(async move {
                uninstalled.store(true, Ordering::SeqCst);
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn test_community_plugin_use_cases_validation_and_delegation() {
        let installed = Arc::new(AtomicBool::new(false));
        let uninstalled = Arc::new(AtomicBool::new(false));
        let mock = MockPluginDownloader {
            installed: Arc::clone(&installed),
            uninstalled: Arc::clone(&uninstalled),
        };
        let use_cases = CommunityPluginUseCases::new(mock);

        // Fetch
        let list = use_cases.fetch_community_plugins().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "test-plugin");

        // Validation: Empty ID
        let empty_res = use_cases
            .install_plugin(Path::new("/vault"), "   ", "author/repo")
            .await;
        assert!(empty_res.is_err());

        // Validation: Invalid repo format
        let invalid_repo = use_cases
            .install_plugin(Path::new("/vault"), "test-plugin", "invalid_repo_without_slash")
            .await;
        assert!(invalid_repo.is_err());

        // Successful install
        let ok_res = use_cases
            .install_plugin(Path::new("/vault"), "test-plugin", "tester/test-plugin")
            .await
            .unwrap();
        assert_eq!(ok_res.id, "test-plugin");
        assert_eq!(ok_res.version, "1.0.0");
        assert!(installed.load(Ordering::SeqCst));

        // Successful uninstall
        use_cases
            .uninstall_plugin(Path::new("/vault"), "test-plugin")
            .await
            .unwrap();
        assert!(uninstalled.load(Ordering::SeqCst));
    }
}

