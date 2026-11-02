//! Sesión de trabajo de una bóveda: pestañas abiertas, modo de vista y estado de la interfaz.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct OpenTabDto {
    pub path: String,
    pub view_mode: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceOpenTabsState {
    pub open_tabs: Vec<OpenTabDto>,
    pub active_tab: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct VaultUiState {
    pub sidebar_width: Option<u32>,
    pub expanded_folders: Vec<String>,
    #[serde(default)]
    pub diff_visual_mode: Option<String>,
    #[serde(default)]
    pub diff_text_mode: Option<String>,
    #[serde(default)]
    pub locked_files: Vec<String>,
}
