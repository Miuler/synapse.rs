use crate::domain::plugin::community_plugin::{
    CommunityPlugin, CommunityPluginInstallResult, InstalledPluginInfo, PluginUpdateInfo,
};
use crate::infrastructure::config::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn fetch_community_plugins(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<CommunityPlugin>, String> {
    state
        .fetch_community_plugins_use_case
        .execute(crate::application::dto::plugin::fetch_community_plugins_query::FetchCommunityPluginsQuery {
            force_refresh: force_refresh.unwrap_or(false),
        })
        .await
}

#[tauri::command(async)]
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
        .get_installed_plugins_use_case
        .execute(crate::application::dto::plugin::get_installed_plugins_query::GetInstalledPluginsQuery { vault_path: resolved_path })
}

#[tauri::command]
pub async fn check_community_plugin_updates(
    state: State<'_, AppState>,
    plugins: Vec<(String, String, String)>,
) -> Result<Vec<PluginUpdateInfo>, String> {
    state
        .check_plugin_updates_use_case
        .execute(crate::application::dto::plugin::check_plugin_updates_query::CheckPluginUpdatesQuery { plugins })
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
        .install_plugin_use_case
        .execute(crate::application::dto::plugin::install_plugin_command::InstallPluginCommand {
            vault_path: resolved_path,
            plugin_id,
            repo,
        })
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
        .uninstall_plugin_use_case
        .execute(crate::application::dto::plugin::uninstall_plugin_command::UninstallPluginCommand {
            vault_path: resolved_path,
            plugin_id,
        })
        .await
}
