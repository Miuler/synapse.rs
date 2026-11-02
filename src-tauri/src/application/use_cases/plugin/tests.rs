//! Pruebas de los casos de uso de plugins comunitarios con un descargador simulado.
use crate::application::dto::plugin::check_plugin_updates_query::CheckPluginUpdatesQuery;
use crate::application::dto::plugin::fetch_community_plugins_query::FetchCommunityPluginsQuery;
use crate::application::dto::plugin::install_plugin_command::InstallPluginCommand;
use crate::application::dto::plugin::uninstall_plugin_command::UninstallPluginCommand;
use crate::application::service::community_plugin_downloader::CommunityPluginDownloader;
use crate::application::use_cases::plugin::check_plugin_updates_use_case::CheckPluginUpdatesUseCase;
use crate::application::use_cases::plugin::fetch_community_plugins_use_case::FetchCommunityPluginsUseCase;
use crate::application::use_cases::plugin::install_plugin_use_case::InstallPluginUseCase;
use crate::application::use_cases::plugin::uninstall_plugin_use_case::UninstallPluginUseCase;
use crate::domain::plugin::community_plugin::{CommunityPlugin, CommunityPluginInstallResult, PluginUpdateInfo};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
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
async fn plugin_use_cases_validate_and_delegate() {
    let installed = Arc::new(AtomicBool::new(false));
    let uninstalled = Arc::new(AtomicBool::new(false));
    let mock = MockPluginDownloader {
        installed: Arc::clone(&installed),
        uninstalled: Arc::clone(&uninstalled),
    };
    let vault = || PathBuf::from("/vault");

    let list = FetchCommunityPluginsUseCase::new(mock.clone())
        .execute(FetchCommunityPluginsQuery { force_refresh: false })
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "test-plugin");

    let updates = CheckPluginUpdatesUseCase::new(mock.clone())
        .execute(CheckPluginUpdatesQuery {
            plugins: vec![("my-cool-plugin".to_string(), "1.2.3".to_string(), "author/plugin".to_string())],
        })
        .await
        .unwrap();
    assert_eq!(updates.len(), 1);
    assert!(updates[0].has_update);

    let install = InstallPluginUseCase::new(mock.clone());
    let cmd = |id: &str, repo: &str| InstallPluginCommand { vault_path: vault(), plugin_id: id.into(), repo: repo.into() };
    assert!(install.execute(cmd("   ", "author/repo")).await.is_err());
    assert!(install.execute(cmd("test-plugin", "invalid_repo_without_slash")).await.is_err());
    let ok_res = install.execute(cmd("test-plugin", "tester/test-plugin")).await.unwrap();
    assert_eq!(ok_res.id, "test-plugin");
    assert_eq!(ok_res.version, "1.0.0");
    assert!(installed.load(Ordering::SeqCst));

    UninstallPluginUseCase::new(mock)
        .execute(UninstallPluginCommand { vault_path: vault(), plugin_id: "test-plugin".into() })
        .await
        .unwrap();
    assert!(uninstalled.load(Ordering::SeqCst));
    let _ = Path::new("/");
}
