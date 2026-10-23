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
