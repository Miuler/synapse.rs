use crate::domain::workspace::workspace_session::{
    OpenTabDto, VaultUiState, WorkspaceOpenTabsState,
};
use crate::infrastructure::config::state::AppState;
use tauri::State;

#[tauri::command]
pub fn record_note_opened(state: State<'_, AppState>, relative_path: String) -> Result<(), String> {
    // Registrar lo abierto es auxiliar: un fallo no debe impedir abrir la nota
    let _ = state.record_note_opened_use_case.execute(
        crate::application::dto::session::record_note_opened_command::RecordNoteOpenedCommand {
            relative_path,
        },
    );
    Ok(())
}

#[tauri::command(async)]
pub fn get_recent_notes_command(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    state.get_recent_notes_use_case.execute(
        crate::application::dto::session::get_recent_notes_query::GetRecentNotesQuery {
            limit: limit.unwrap_or(15),
        },
    )
}

#[tauri::command]
pub fn save_open_tabs_state(
    state: State<'_, AppState>,
    tabs: Vec<OpenTabDto>,
    active_tab: Option<String>,
) -> Result<(), String> {
    state.save_open_tabs_use_case.execute(
        crate::application::dto::session::save_open_tabs_command::SaveOpenTabsCommand {
            tabs,
            active_tab,
        },
    )
}

#[tauri::command(async)]
pub fn get_open_tabs_state(state: State<'_, AppState>) -> Result<WorkspaceOpenTabsState, String> {
    state
        .get_open_tabs_use_case
        .execute(crate::application::dto::session::get_open_tabs_query::GetOpenTabsQuery {})
}

#[tauri::command]
pub fn set_note_view_mode(
    state: State<'_, AppState>,
    relative_path: String,
    view_mode: String,
) -> Result<(), String> {
    state.set_note_view_mode_use_case.execute(
        crate::application::dto::session::set_note_view_mode_command::SetNoteViewModeCommand {
            relative_path,
            view_mode,
        },
    )
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
    state.save_vault_ui_state_use_case.execute(
        crate::application::dto::session::save_vault_ui_state_command::SaveVaultUiStateCommand {
            sidebar_width,
            expanded_folders,
            diff_visual_mode,
            diff_text_mode,
            locked_files,
        },
    )
}

#[tauri::command(async)]
pub fn get_vault_ui_state(state: State<'_, AppState>) -> Result<VaultUiState, String> {
    state.get_vault_ui_state_use_case.execute(
        crate::application::dto::session::get_vault_ui_state_query::GetVaultUiStateQuery {},
    )
}

/// Límites (porcentaje del ancho) permitidos para el panel de código de un diagrama.
const DIAGRAM_SPLIT_MIN_PCT: f32 = 15.0;
const DIAGRAM_SPLIT_MAX_PCT: f32 = 85.0;

/// Devuelve la proporción recordada del panel de código de un diagrama, si el usuario la modificó.
#[tauri::command(async)]
pub fn get_diagram_split_ratio(state: State<'_, AppState>, path: String) -> Option<f32> {
    state.diagram_split_ratios.get(&path).map(|r| *r.value())
}

/// Recuerda en memoria (sin persistir a disco) la proporción del panel de código de un diagrama.
#[tauri::command]
pub fn set_diagram_split_ratio(state: State<'_, AppState>, path: String, ratio: f32) {
    if ratio.is_finite() {
        state.diagram_split_ratios.insert(
            path,
            ratio.clamp(DIAGRAM_SPLIT_MIN_PCT, DIAGRAM_SPLIT_MAX_PCT),
        );
    }
}
