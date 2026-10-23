use crate::domain::models::community_plugin::{
    CommunityPlugin, CommunityPluginInstallResult, InstalledPluginInfo, PluginUpdateInfo,
};
use crate::domain::services::community_plugin_downloader::CommunityPluginDownloader;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ManifestExtract {
    id: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    author: Option<String>,
}

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

    pub async fn fetch_community_plugins(&self, force_refresh: bool) -> Result<Vec<CommunityPlugin>, String> {
        self.downloader.fetch_community_plugins(force_refresh).await
    }

    pub fn get_installed_plugins(&self, vault_path: &Path) -> Result<Vec<InstalledPluginInfo>, String> {
        let mut results = Vec::new();
        let mut seen_ids = HashSet::new();

        let search_dirs = [
            (".synapse/plugins", vault_path.join(".synapse").join("plugins")),
            (".obsidian/plugins", vault_path.join(".obsidian").join("plugins")),
        ];

        for (rel_prefix, abs_dir) in &search_dirs {
            if !abs_dir.exists() || !abs_dir.is_dir() {
                continue;
            }

            let entries = match fs::read_dir(abs_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let entry_path = entry.path();
                if !entry_path.is_dir() {
                    continue;
                }

                let folder_name = entry.file_name().to_string_lossy().to_string();
                let manifest_path = entry_path.join("manifest.json");

                if manifest_path.exists() {
                    if let Ok(content) = fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<ManifestExtract>(&content) {
                            let id = manifest.id.unwrap_or_else(|| folder_name.clone());
                            if seen_ids.contains(&id) {
                                continue;
                            }
                            seen_ids.insert(id.clone());

                            let name = manifest.name.unwrap_or_else(|| id.clone());
                            let version = manifest.version.unwrap_or_else(|| "1.0.0".to_string());
                            let has_styles = entry_path.join("styles.css").exists();
                            let dir = format!("{rel_prefix}/{folder_name}");

                            results.push(InstalledPluginInfo {
                                id,
                                name,
                                version,
                                description: manifest.description,
                                author: manifest.author,
                                dir,
                                has_styles,
                                repo: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    pub async fn check_plugin_updates(
        &self,
        plugins: &[(String, String, String)],
    ) -> Result<Vec<PluginUpdateInfo>, String> {
        self.downloader.check_plugin_updates(plugins).await
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
    use tempfile::tempdir;

    struct MockPluginDownloader {
        installed: Arc<AtomicBool>,
        uninstalled: Arc<AtomicBool>,
    }

    impl CommunityPluginDownloader for MockPluginDownloader {
        fn fetch_community_plugins(
            &self,
            _force_refresh: bool,
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

        fn check_plugin_updates<'a>(
            &'a self,
            plugins: &'a [(String, String, String)],
        ) -> Pin<Box<dyn Future<Output = Result<Vec<PluginUpdateInfo>, String>> + Send + 'a>> {
            Box::pin(async move {
                let updates = plugins
                    .iter()
                    .map(|(id, current_version, repo)| PluginUpdateInfo {
                        id: id.clone(),
                        current_version: current_version.clone(),
                        latest_version: "2.0.0".to_string(),
                        has_update: true,
                        repo: repo.clone(),
                    })
                    .collect();
                Ok(updates)
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
        let list = use_cases.fetch_community_plugins(false).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "test-plugin");

        // Installed plugins scan with temp dir
        let temp = tempdir().unwrap();
        let plugin_dir = temp.path().join(".synapse").join("plugins").join("my-cool-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("manifest.json"),
            r#"{"id":"my-cool-plugin","name":"My Cool Plugin","version":"1.2.3"}"#,
        )
        .unwrap();

        let scanned = use_cases.get_installed_plugins(temp.path()).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].id, "my-cool-plugin");
        assert_eq!(scanned[0].name, "My Cool Plugin");
        assert_eq!(scanned[0].version, "1.2.3");

        // Updates
        let updates = use_cases
            .check_plugin_updates(&[("my-cool-plugin".to_string(), "1.2.3".to_string(), "author/plugin".to_string())])
            .await
            .unwrap();
        assert_eq!(updates.len(), 1);
        assert!(updates[0].has_update);

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

