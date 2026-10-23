use crate::domain::models::community_plugin::{
    CommunityPlugin, CommunityPluginInstallResult, PluginUpdateInfo,
};
use crate::domain::services::community_plugin_downloader::CommunityPluginDownloader;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

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

fn parse_version_parts(v: &str) -> Vec<u64> {
    let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
    clean
        .split(|c: char| c == '.' || c == '-' || c == '+')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect()
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_parts = parse_version_parts(latest);
    let current_parts = parse_version_parts(current);

    if latest_parts.is_empty() || current_parts.is_empty() {
        return latest != current && !latest.is_empty();
    }

    let max_len = latest_parts.len().max(current_parts.len());
    for i in 0..max_len {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }
    false
}

fn get_disk_cache_path() -> PathBuf {
    std::env::temp_dir().join("synapse_community_plugins_cache.json")
}

pub struct HttpCommunityPluginDownloader {
    client: reqwest::Client,
    cache: Arc<RwLock<Option<(Instant, Vec<CommunityPlugin>)>>>,
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

        Self {
            client,
            cache: Arc::new(RwLock::new(None)),
        }
    }
}

impl CommunityPluginDownloader for HttpCommunityPluginDownloader {
    fn fetch_community_plugins(
        &self,
        force_refresh: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CommunityPlugin>, String>> + Send + '_>> {
        let cache_arc = Arc::clone(&self.cache);
        let client = self.client.clone();

        Box::pin(async move {
            let cache_file = get_disk_cache_path();

            // Si no se fuerza recarga, intentar leer de memoria primero
            if !force_refresh {
                if let Ok(guard) = cache_arc.read() {
                    if let Some((instant, ref list)) = *guard {
                        // Caché en memoria válida por 6 horas
                        if instant.elapsed() < Duration::from_secs(3600 * 6) {
                            return Ok(list.clone());
                        }
                    }
                }

                // Fallback a caché en disco persistente
                if cache_file.exists() {
                    if let Ok(bytes) = fs::read(&cache_file) {
                        if let Ok(cached_plugins) = serde_json::from_slice::<Vec<CommunityPlugin>>(&bytes) {
                            if !cached_plugins.is_empty() {
                                if let Ok(mut guard) = cache_arc.write() {
                                    *guard = Some((Instant::now(), cached_plugins.clone()));
                                }
                                return Ok(cached_plugins);
                            }
                        }
                    }
                }
            }

            // 1. Descargar catálogo de plugins desde GitHub
            let res = client
                .get(COMMUNITY_PLUGINS_URL)
                .send()
                .await
                .map_err(|e| format!("Error de conexión al obtener catálogo de plugins: {e}"))?;

            if !res.status().is_success() {
                // Si falla la red pero tenemos caché en disco previa, devolverla como salvaguarda
                if cache_file.exists() {
                    if let Ok(bytes) = fs::read(&cache_file) {
                        if let Ok(cached_plugins) = serde_json::from_slice::<Vec<CommunityPlugin>>(&bytes) {
                            if !cached_plugins.is_empty() {
                                return Ok(cached_plugins);
                            }
                        }
                    }
                }
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
            if let Ok(stats_res) = client.get(COMMUNITY_PLUGIN_STATS_URL).send().await {
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

            // Actualizar caché en memoria
            if let Ok(mut guard) = cache_arc.write() {
                *guard = Some((Instant::now(), plugins.clone()));
            }

            // Actualizar caché en disco
            if let Ok(serialized) = serde_json::to_vec(&plugins) {
                let _ = fs::write(&cache_file, serialized);
            }

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

    fn check_plugin_updates<'a>(
        &'a self,
        plugins: &'a [(String, String, String)],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PluginUpdateInfo>, String>> + Send + 'a>> {
        Box::pin(async move {
            let mut results = Vec::new();

            for (id, current_version, repo) in plugins {
                let manifest_urls = [
                    format!("https://raw.githubusercontent.com/{repo}/HEAD/manifest.json"),
                    format!("https://raw.githubusercontent.com/{repo}/master/manifest.json"),
                    format!("https://raw.githubusercontent.com/{repo}/main/manifest.json"),
                    format!("https://github.com/{repo}/releases/latest/download/manifest.json"),
                ];

                let mut latest_version = current_version.clone();
                for url in &manifest_urls {
                    if let Ok(res) = self
                        .client
                        .get(url)
                        .timeout(Duration::from_secs(6))
                        .send()
                        .await
                    {
                        if res.status().is_success() {
                            if let Ok(parsed) = res.json::<ManifestVersionExtract>().await {
                                if let Some(v) = parsed.version {
                                    latest_version = v;
                                    break;
                                }
                            }
                        }
                    }
                }

                let has_update = is_newer_version(&latest_version, current_version);
                results.push(PluginUpdateInfo {
                    id: id.clone(),
                    current_version: current_version.clone(),
                    latest_version,
                    has_update,
                    repo: repo.clone(),
                });
            }

            Ok(results)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version_comparisons() {
        assert!(is_newer_version("1.2.0", "1.1.9"));
        assert!(is_newer_version("2.0.0", "1.99.99"));
        assert!(is_newer_version("1.10.0", "1.9.0"));
        assert!(is_newer_version("v1.2.3", "1.2.2"));
        assert!(is_newer_version("1.2.3-beta.2", "1.2.2"));

        assert!(!is_newer_version("1.2.0", "1.2.0"));
        assert!(!is_newer_version("1.1.9", "1.2.0"));
        assert!(!is_newer_version("0.9.0", "1.0.0"));
    }
}
