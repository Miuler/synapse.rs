use crate::domain::models::community_plugin::{
    CommunityPlugin, CommunityPluginInstallResult, InstalledPluginInfo, PluginUpdateInfo,
};
use crate::infrastructure::tauri::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn fetch_community_plugins(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<CommunityPlugin>, String> {
    state
        .community_plugin_use_cases
        .fetch_community_plugins(force_refresh.unwrap_or(false))
        .await
}

#[tauri::command]
pub fn get_installed_community_plugins(
    state: State<'_, AppState>,
    vault_path: Option<String>,
) -> Result<Vec<InstalledPluginInfo>, String> {
    let resolved_path = match vault_path {
        Some(p) => PathBuf::from(p),
        None => state
            .active_vault_path
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No hay ninguna bóveda activa seleccionada".to_string())?,
    };

    state
        .community_plugin_use_cases
        .get_installed_plugins(&resolved_path)
}

#[tauri::command]
pub async fn check_community_plugin_updates(
    state: State<'_, AppState>,
    plugins: Vec<(String, String, String)>,
) -> Result<Vec<PluginUpdateInfo>, String> {
    state
        .community_plugin_use_cases
        .check_plugin_updates(&plugins)
        .await
}

#[tauri::command]
pub async fn install_community_plugin(
    state: State<'_, AppState>,
    vault_path: Option<String>,
    plugin_id: String,
    repo: String,
) -> Result<CommunityPluginInstallResult, String> {
    let resolved_path = match vault_path {
        Some(p) => PathBuf::from(p),
        None => state
            .active_vault_path
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No hay ninguna bóveda activa seleccionada".to_string())?,
    };

    state
        .community_plugin_use_cases
        .install_plugin(&resolved_path, &plugin_id, &repo)
        .await
}

#[tauri::command]
pub async fn uninstall_community_plugin(
    state: State<'_, AppState>,
    vault_path: Option<String>,
    plugin_id: String,
) -> Result<(), String> {
    let resolved_path = match vault_path {
        Some(p) => PathBuf::from(p),
        None => state
            .active_vault_path
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No hay ninguna bóveda activa seleccionada".to_string())?,
    };

    state
        .community_plugin_use_cases
        .uninstall_plugin(&resolved_path, &plugin_id)
        .await
}
