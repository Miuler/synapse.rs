use crate::application::dto::appearance::{
    GetThemeDirsQuery, ListGlobalThemesQuery, SaveThemeDirsCommand,
};
use crate::domain::appearance::GlobalTheme;
use crate::infrastructure::config::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_theme_dirs(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.get_theme_dirs_use_case.execute(GetThemeDirsQuery {})
}

#[tauri::command]
pub async fn save_theme_dirs(
    state: State<'_, AppState>,
    dirs: Vec<String>,
) -> Result<Vec<String>, String> {
    state
        .save_theme_dirs_use_case
        .execute(SaveThemeDirsCommand { dirs })
}

#[tauri::command]
pub async fn list_global_themes(state: State<'_, AppState>) -> Result<Vec<GlobalTheme>, String> {
    state
        .list_global_themes_use_case
        .execute(ListGlobalThemesQuery {})
}
