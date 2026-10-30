use crate::infrastructure::navigation::engine::{OpenTabDto, VaultUiState, WorkspaceOpenTabsState};
use crate::infrastructure::tauri::state::AppState;
use tauri::State;

#[tauri::command]
pub fn record_note_opened(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.record_opened(&relative_path);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_recent_notes_command(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(Vec::new());
    };
    let recent = engine.get_recent_notes(limit.unwrap_or(15));
    Ok(recent.into_iter().map(|n| n.path.to_string()).collect())
}

#[tauri::command]
pub fn save_open_tabs_state(
    state: State<'_, AppState>,
    tabs: Vec<OpenTabDto>,
    active_tab: Option<String>,
) -> Result<(), String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *guard {
        engine.save_open_tabs_state(&tabs, active_tab.as_deref());
    }
    Ok(())
}

#[tauri::command]
pub fn get_open_tabs_state(
    state: State<'_, AppState>,
) -> Result<WorkspaceOpenTabsState, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(WorkspaceOpenTabsState {
            open_tabs: Vec::new(),
            active_tab: None,
        });
    };
    Ok(engine.get_open_tabs_state())
}

#[tauri::command]
pub fn set_note_view_mode(
    state: State<'_, AppState>,
    relative_path: String,
    view_mode: String,
) -> Result<(), String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *guard {
        engine.set_note_view_mode(&relative_path, &view_mode);
    }
    Ok(())
}

#[tauri::command]
pub fn save_vault_ui_state(
    state: State<'_, AppState>,
    sidebar_width: Option<u32>,
    expanded_folders: Option<Vec<String>>,
    diff_visual_mode: Option<String>,
    diff_text_mode: Option<String>,
    locked_files: Option<Vec<String>>,
) -> Result<(), String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *guard {
        engine.save_vault_ui_state(
            sidebar_width,
            expanded_folders,
            diff_visual_mode,
            diff_text_mode,
            locked_files,
        );
    }
    Ok(())
}

#[tauri::command]
pub fn get_vault_ui_state(
    state: State<'_, AppState>,
) -> Result<VaultUiState, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(VaultUiState::default());
    };
    Ok(engine.get_vault_ui_state())
}

/// Límites (porcentaje del ancho) permitidos para el panel de código de un diagrama.
const DIAGRAM_SPLIT_MIN_PCT: f32 = 15.0;
const DIAGRAM_SPLIT_MAX_PCT: f32 = 85.0;

/// Devuelve la proporción recordada del panel de código de un diagrama, si el usuario la modificó.
#[tauri::command]
pub fn get_diagram_split_ratio(state: State<'_, AppState>, path: String) -> Option<f32> {
    state.diagram_split_ratios.get(&path).map(|r| *r.value())
}

/// Recuerda en memoria (sin persistir a disco) la proporción del panel de código de un diagrama.
#[tauri::command]
pub fn set_diagram_split_ratio(state: State<'_, AppState>, path: String, ratio: f32) {
    if ratio.is_finite() {
        state
            .diagram_split_ratios
            .insert(path, ratio.clamp(DIAGRAM_SPLIT_MIN_PCT, DIAGRAM_SPLIT_MAX_PCT));
    }
}
