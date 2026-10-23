use serde::{Deserialize, Serialize};

/// Representa un plugin comunitario publicado en el directorio oficial de Obsidian.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunityPlugin {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub repo: String,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub downloads: Option<u64>,
}

/// Resultado tras la instalación o actualización exitosa de un plugin comunitario.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunityPluginInstallResult {
    pub id: String,
    pub version: String,
    pub installed_path: String,
    pub has_styles: bool,
}

/// Información sobre un plugin instalado detectado en el disco de la bóveda.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledPluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub dir: String,
    pub has_styles: bool,
    #[serde(default)]
    pub repo: Option<String>,
}

/// Información sobre actualización disponible para un plugin instalado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginUpdateInfo {
    pub id: String,
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub repo: String,
}

/// Listado de archivos y subdirectorios en una ruta física de la bóveda.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultDirectoryList {
    pub files: Vec<String>,
    pub folders: Vec<String>,
}
