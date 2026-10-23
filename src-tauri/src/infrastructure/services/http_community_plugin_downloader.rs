use crate::domain::models::community_plugin::{CommunityPlugin, CommunityPluginInstallResult};
use crate::domain::services::community_plugin_downloader::CommunityPluginDownloader;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

const COMMUNITY_PLUGINS_URL: &str =
    "https://raw.githubusercontent.com/obsidianmd/obsidian-releases/master/community-plugins.json";
const COMMUNITY_PLUGIN_STATS_URL: &str =
    "https://raw.githubusercontent.com/obsidianmd/obsidian-releases/master/community-plugin-stats.json";

#[derive(Debug, Deserialize)]
struct PluginStatItem {
    downloads: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ManifestVersionExtract {
    version: Option<String>,
}

pub struct HttpCommunityPluginDownloader {
    client: reqwest::Client,
}

impl Default for HttpCommunityPluginDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpCommunityPluginDownloader {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Synapse-App/0.1.0 (https://github.com/Miuler/synapse.rs)")
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }
}

impl CommunityPluginDownloader for HttpCommunityPluginDownloader {
    fn fetch_community_plugins(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CommunityPlugin>, String>> + Send + '_>> {
        Box::pin(async move {
            // 1. Descargar catálogo de plugins
            let res = self
                .client
                .get(COMMUNITY_PLUGINS_URL)
                .send()
                .await
                .map_err(|e| format!("Error de conexión al obtener catálogo de plugins: {e}"))?;

            if !res.status().is_success() {
                return Err(format!(
                    "Respuesta fallida de GitHub al obtener catálogo: HTTP {}",
                    res.status()
                ));
            }

            let mut plugins: Vec<CommunityPlugin> = res
                .json()
                .await
                .map_err(|e| format!("Error al decodificar catálogo de plugins: {e}"))?;

            // 2. Descargar estadísticas de descargas de forma opcional
            if let Ok(stats_res) = self.client.get(COMMUNITY_PLUGIN_STATS_URL).send().await {
                if stats_res.status().is_success() {
                    if let Ok(stats) = stats_res.json::<HashMap<String, PluginStatItem>>().await {
                        for p in plugins.iter_mut() {
                            if let Some(st) = stats.get(&p.id) {
                                p.downloads = st.downloads;
                            }
                        }
                    }
                }
            }

            // Ordenar por popularidad (más descargas primero)
            plugins.sort_by(|a, b| b.downloads.unwrap_or(0).cmp(&a.downloads.unwrap_or(0)));

            Ok(plugins)
        })
    }

    fn install_plugin<'a>(
        &'a self,
        vault_path: &'a Path,
        plugin_id: &'a str,
        repo: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CommunityPluginInstallResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let target_dir = vault_path.join(".synapse").join("plugins").join(plugin_id);
            fs::create_dir_all(&target_dir).map_err(|e| {
                format!(
                    "No se pudo crear el directorio de plugin en '{}': {e}",
                    target_dir.display()
                )
            })?;

            // 1. Descargar manifest.json
            let manifest_urls = [
                format!("https://github.com/{repo}/releases/latest/download/manifest.json"),
                format!("https://raw.githubusercontent.com/{repo}/HEAD/manifest.json"),
                format!("https://raw.githubusercontent.com/{repo}/master/manifest.json"),
                format!("https://raw.githubusercontent.com/{repo}/main/manifest.json"),
            ];

            let mut manifest_bytes: Option<Vec<u8>> = None;
            for url in &manifest_urls {
                if let Ok(res) = self.client.get(url).send().await {
                    if res.status().is_success() {
                        if let Ok(bytes) = res.bytes().await {
                            if !bytes.is_empty() {
                                manifest_bytes = Some(bytes.to_vec());
                                break;
                            }
                        }
                    }
                }
            }

            let manifest_bytes = manifest_bytes.ok_or_else(|| {
                format!("No se pudo descargar 'manifest.json' para el plugin '{plugin_id}' desde {repo}")
            })?;

            // Extraer versión de manifest.json
            let mut detected_version = "latest".to_string();
            if let Ok(parsed) = serde_json::from_slice::<ManifestVersionExtract>(&manifest_bytes) {
                if let Some(v) = parsed.version {
                    detected_version = v;
                }
            }

            // Guardar manifest.json
            let manifest_path = target_dir.join("manifest.json");
            fs::write(&manifest_path, &manifest_bytes).map_err(|e| {
                format!("Error al escribir 'manifest.json' en disco: {e}")
            })?;

            // 2. Descargar main.js
            let main_urls = [
                format!("https://github.com/{repo}/releases/latest/download/main.js"),
                format!("https://github.com/{repo}/releases/download/{detected_version}/main.js"),
                format!("https://github.com/{repo}/releases/download/v{detected_version}/main.js"),
                format!("https://raw.githubusercontent.com/{repo}/HEAD/main.js"),
                format!("https://raw.githubusercontent.com/{repo}/master/main.js"),
            ];

            let mut main_bytes: Option<Vec<u8>> = None;
            for url in &main_urls {
                if let Ok(res) = self.client.get(url).send().await {
                    if res.status().is_success() {
                        if let Ok(bytes) = res.bytes().await {
                            if !bytes.is_empty() {
                                main_bytes = Some(bytes.to_vec());
                                break;
                            }
                        }
                    }
                }
            }

            let main_bytes = main_bytes.ok_or_else(|| {
                format!("No se pudo descargar 'main.js' para el plugin '{plugin_id}' desde los releases de {repo}")
            })?;

            let main_path = target_dir.join("main.js");
            fs::write(&main_path, &main_bytes).map_err(|e| {
                format!("Error al escribir 'main.js' en disco: {e}")
            })?;

            // 3. Descargar styles.css (opcional)
            let styles_urls = [
                format!("https://github.com/{repo}/releases/latest/download/styles.css"),
                format!("https://github.com/{repo}/releases/download/{detected_version}/styles.css"),
                format!("https://github.com/{repo}/releases/download/v{detected_version}/styles.css"),
                format!("https://raw.githubusercontent.com/{repo}/HEAD/styles.css"),
                format!("https://raw.githubusercontent.com/{repo}/master/styles.css"),
            ];

            let mut has_styles = false;
            for url in &styles_urls {
                if let Ok(res) = self.client.get(url).send().await {
                    if res.status().is_success() {
                        if let Ok(bytes) = res.bytes().await {
                            if !bytes.is_empty() {
                                let styles_path = target_dir.join("styles.css");
                                if fs::write(&styles_path, &bytes).is_ok() {
                                    has_styles = true;
                                }
                                break;
                            }
                        }
                    }
                }
            }

            Ok(CommunityPluginInstallResult {
                id: plugin_id.to_string(),
                version: detected_version,
                installed_path: target_dir.to_string_lossy().to_string(),
                has_styles,
            })
        })
    }

    fn uninstall_plugin<'a>(
        &'a self,
        vault_path: &'a Path,
        plugin_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let mut removed = false;

            // 1. Eliminar de .synapse/plugins/{id}
            let synapse_plugin_dir = vault_path.join(".synapse").join("plugins").join(plugin_id);
            if synapse_plugin_dir.exists() {
                fs::remove_dir_all(&synapse_plugin_dir).map_err(|e| {
                    format!(
                        "Error al eliminar plugin de '{}': {e}",
                        synapse_plugin_dir.display()
                    )
                })?;
                removed = true;
            }

            // 2. Eliminar de .obsidian/plugins/{id} si existe
            let obsidian_plugin_dir = vault_path.join(".obsidian").join("plugins").join(plugin_id);
            if obsidian_plugin_dir.exists() {
                let _ = fs::remove_dir_all(&obsidian_plugin_dir);
                removed = true;
            }

            if !removed {
                return Err(format!(
                    "No se encontró el plugin '{plugin_id}' en .synapse/plugins ni en .obsidian/plugins"
                ));
            }

            Ok(())
        })
    }
}
