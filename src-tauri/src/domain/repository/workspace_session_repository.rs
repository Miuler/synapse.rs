use crate::domain::model::workspace_session::{OpenTabDto, VaultUiState, WorkspaceOpenTabsState};

/// Sesión de trabajo de la bóveda activa: notas recientes, pestañas, modo de vista de cada nota
/// y estado de la interfaz. Sin bóveda abierta, las lecturas devuelven valores vacíos, salvo
/// `view_mode`, que devuelve error para no pisar el modo de vista leído de la nota.
pub trait WorkspaceSessionRepository: Send + Sync {
    fn record_opened(&self, relative_path: &str) -> Result<(), String>;
    fn recent_note_paths(&self, limit: usize) -> Result<Vec<String>, String>;
    fn save_open_tabs(&self, tabs: &[OpenTabDto], active_tab: Option<&str>) -> Result<(), String>;
    fn open_tabs(&self) -> Result<WorkspaceOpenTabsState, String>;
    fn set_view_mode(&self, relative_path: &str, view_mode: &str) -> Result<(), String>;
    fn view_mode(&self, relative_path: &str) -> Result<Option<String>, String>;
    fn save_ui_state(
        &self,
        sidebar_width: Option<u32>,
        expanded_folders: Option<Vec<String>>,
        diff_visual_mode: Option<String>,
        diff_text_mode: Option<String>,
        locked_files: Option<Vec<String>>,
    ) -> Result<(), String>;
    fn ui_state(&self) -> Result<VaultUiState, String>;
}
