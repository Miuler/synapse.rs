use crate::domain::model::workspace_session::{OpenTabDto, VaultUiState, WorkspaceOpenTabsState};
use crate::domain::repository::workspace_session_repository::WorkspaceSessionRepository;
use crate::infrastructure::navigation::engine::NavigationEngine;
use std::sync::{Arc, Mutex};

/// Sesión de trabajo guardada en el motor de navegación de la bóveda activa (su DashMap y su
/// caché en disco). Comparte el motor con el resto de la app: no duplica datos.
#[derive(Clone)]
pub struct WorkspaceSessionRepositoryImpl {
    engine: Arc<Mutex<Option<Arc<NavigationEngine>>>>,
}

impl WorkspaceSessionRepositoryImpl {
    pub fn new(engine: Arc<Mutex<Option<Arc<NavigationEngine>>>>) -> Self {
        Self { engine }
    }

    fn with_engine<T>(&self, default: T, f: impl FnOnce(&NavigationEngine) -> T) -> Result<T, String> {
        let guard = self.engine.lock().map_err(|e| e.to_string())?;
        Ok(match *guard {
            Some(ref engine) => f(engine),
            None => default,
        })
    }
}

impl WorkspaceSessionRepository for WorkspaceSessionRepositoryImpl {
    fn record_opened(&self, relative_path: &str) -> Result<(), String> {
        self.with_engine((), |e| e.record_opened(relative_path))
    }

    fn recent_note_paths(&self, limit: usize) -> Result<Vec<String>, String> {
        self.with_engine(Vec::new(), |e| {
            e.get_recent_notes(limit).into_iter().map(|n| n.path.to_string()).collect()
        })
    }

    fn save_open_tabs(&self, tabs: &[OpenTabDto], active_tab: Option<&str>) -> Result<(), String> {
        self.with_engine((), |e| e.save_open_tabs_state(tabs, active_tab))
    }

    fn open_tabs(&self) -> Result<WorkspaceOpenTabsState, String> {
        self.with_engine(
            WorkspaceOpenTabsState {
                open_tabs: Vec::new(),
                active_tab: None,
            },
            |e| e.get_open_tabs_state(),
        )
    }

    fn set_view_mode(&self, relative_path: &str, view_mode: &str) -> Result<(), String> {
        self.with_engine((), |e| e.set_note_view_mode(relative_path, view_mode))
    }

    fn view_mode(&self, relative_path: &str) -> Result<Option<String>, String> {
        self.with_engine(Err("No hay ninguna bóveda abierta".to_string()), |e| Ok(e.get_note_view_mode(relative_path)))?
    }

    fn save_ui_state(
        &self,
        sidebar_width: Option<u32>,
        expanded_folders: Option<Vec<String>>,
        diff_visual_mode: Option<String>,
        diff_text_mode: Option<String>,
        locked_files: Option<Vec<String>>,
    ) -> Result<(), String> {
        self.with_engine((), |e| {
            e.save_vault_ui_state(sidebar_width, expanded_folders, diff_visual_mode, diff_text_mode, locked_files)
        })
    }

    fn ui_state(&self) -> Result<VaultUiState, String> {
        self.with_engine(VaultUiState::default(), |e| e.get_vault_ui_state())
    }
}
