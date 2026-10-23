use crate::domain::models::community_plugin::{
    CommunityPlugin, CommunityPluginInstallResult, PluginUpdateInfo,
};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

/// Trait / Puerto para descargar e instalar plugins comunitarios de Obsidian.
///
/// Permite desacoplar el cliente HTTP y el acceso a disco de la lógica de dominio.
pub trait CommunityPluginDownloader: Send + Sync {
    /// Obtiene la lista completa de plugins comunitarios desde el catálogo oficial.
    fn fetch_community_plugins(
        &self,
        force_refresh: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<CommunityPlugin>, String>> + Send + '_>>;

    /// Descarga los assets de un plugin (manifest.json, main.js, styles.css) y los instala en la bóveda.
    fn install_plugin<'a>(
        &'a self,
        vault_path: &'a Path,
        plugin_id: &'a str,
        repo: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CommunityPluginInstallResult, String>> + Send + 'a>>;

    /// Desinstala un plugin eliminando su directorio en `.synapse/plugins/{id}` (o `.obsidian/plugins/{id}`).
    fn uninstall_plugin<'a>(
        &'a self,
        vault_path: &'a Path,
        plugin_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

    /// Comprueba si hay versiones más recientes para una lista de plugins instalados (id, current_version, repo).
    fn check_plugin_updates<'a>(
        &'a self,
        plugins: &'a [(String, String, String)],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PluginUpdateInfo>, String>> + Send + 'a>>;
}
