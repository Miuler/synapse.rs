use crate::application::dto::version::checkout_branch_command::CheckoutBranchCommand;
use crate::application::dto::version::create_branch_command::CreateBranchCommand;
use crate::application::dto::version::get_branches_query::GetBranchesQuery;
use crate::application::dto::version::get_file_diff_query::GetFileDiffQuery;
use crate::application::dto::version::get_file_revision_query::GetFileRevisionQuery;
use crate::application::dto::version::get_file_version_history_query::GetFileVersionHistoryQuery;
use crate::application::dto::version::get_image_diff_query::GetImageDiffQuery;
use crate::application::dto::version::get_vault_version_state_query::GetVaultVersionStateQuery;
use crate::application::dto::version::push_versions_command::PushVersionsCommand;
use crate::application::dto::version::revert_files_command::RevertFilesCommand;
use crate::application::dto::version::save_version_command::SaveVersionCommand;
use crate::application::dto::version::track_files_command::TrackFilesCommand;
use crate::application::dto::version::untrack_files_command::UntrackFilesCommand;
use crate::domain::version::version::{
    BranchList, FileDiff, FileRevision, FileVersionHistory, ImageDiff, VaultVersionState,
};
use crate::infrastructure::config::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn get_vault_git_status(
    state: State<'_, AppState>,
    folder_path: Option<String>,
) -> Result<VaultVersionState, String> {
    let vault_path = match folder_path {
        Some(p) => PathBuf::from(p),
        None => {
            let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
            match *guard {
                Some(ref p) => p.clone(),
                None => {
                    return Ok(VaultVersionState {
                        is_repo: false,
                        branch: None,
                        statuses: Default::default(),
                    });
                }
            }
        }
    };
    let use_case = state.get_vault_version_state_use_case.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use_case.execute(GetVaultVersionStateQuery { vault_path })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn git_add_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .track_files_use_case
        .execute(TrackFilesCommand {
            vault_path: vault_path.clone(),
            paths,
        })
        .map_err(|err| {
            log::error!("[commands::git_add_paths] {}", err);
            eprintln!("[commands::git_add_paths] {}", err);
            err
        })
}

#[tauri::command]
pub fn git_restore_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .revert_files_use_case
        .execute(RevertFilesCommand {
            vault_path: vault_path.clone(),
            paths,
        })
        .map_err(|err| {
            log::error!("[commands::git_restore_paths] {}", err);
            eprintln!("[commands::git_restore_paths] {}", err);
            err
        })
}

#[tauri::command]
pub fn git_restore_staged_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .untrack_files_use_case
        .execute(UntrackFilesCommand {
            vault_path: vault_path.clone(),
            paths,
        })
        .map_err(|err| {
            log::error!("[commands::git_restore_staged_paths] {}", err);
            eprintln!("[commands::git_restore_staged_paths] {}", err);
            err
        })
}

#[tauri::command]
pub fn git_commit_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
    message: String,
) -> Result<String, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .save_version_use_case
        .execute(SaveVersionCommand {
            vault_path: vault_path.clone(),
            paths,
            message,
        })
        .map_err(|err| {
            log::error!("[commands::git_commit_paths] {}", err);
            eprintln!("[commands::git_commit_paths] {}", err);
            err
        })
}

#[tauri::command(async)]
pub fn get_git_file_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<FileDiff, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .get_file_diff_use_case
        .execute(GetFileDiffQuery {
            vault_path: vault_path.clone(),
            path,
            staged,
            revision,
        })
        .map_err(|err| {
            log::error!("[commands::get_git_file_diff] {}", err);
            eprintln!("[commands::get_git_file_diff] {}", err);
            err
        })
}

#[tauri::command(async)]
pub fn get_git_file_revision(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<FileRevision, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .get_file_revision_use_case
        .execute(GetFileRevisionQuery {
            vault_path: vault_path.clone(),
            path,
            staged,
            revision,
        })
        .map_err(|err| {
            log::error!("[commands::get_git_file_revision] {}", err);
            eprintln!("[commands::get_git_file_revision] {}", err);
            err
        })
}

#[tauri::command(async)]
pub fn get_git_image_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<ImageDiff, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .get_image_diff_use_case
        .execute(GetImageDiffQuery {
            vault_path: vault_path.clone(),
            path,
            staged,
            revision,
        })
        .map_err(|err| {
            log::error!("[commands::get_git_image_diff] {}", err);
            eprintln!("[commands::get_git_image_diff] {}", err);
            err
        })
}

#[tauri::command(async)]
pub fn get_git_file_log(
    state: State<'_, AppState>,
    path: String,
    max_count: Option<usize>,
) -> Result<FileVersionHistory, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .get_file_version_history_use_case
        .execute(GetFileVersionHistoryQuery {
            vault_path: vault_path.clone(),
            path,
            max_count,
        })
        .map_err(|err| {
            log::error!("[commands::get_git_file_log] {}", err);
            eprintln!("[commands::get_git_file_log] {}", err);
            err
        })
}

#[tauri::command(async)]
pub fn get_git_branches(state: State<'_, AppState>) -> Result<BranchList, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state.get_branches_use_case.execute(GetBranchesQuery {
        vault_path: vault_path.clone(),
    })
}

#[tauri::command]
pub fn git_checkout_branch(
    state: State<'_, AppState>,
    branch_name: String,
) -> Result<String, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let result = state
        .checkout_branch_use_case
        .execute(CheckoutBranchCommand {
            vault_path: vault_path.clone(),
            branch_name,
        })?;

    // Sincronizar el árbol de navegación ya que los archivos cambiaron en disco
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.spawn_reconciliation();
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn git_create_branch(
    state: State<'_, AppState>,
    new_branch: String,
    base_branch: String,
    checkout: bool,
) -> Result<String, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let result = state.create_branch_use_case.execute(CreateBranchCommand {
        vault_path: vault_path.clone(),
        new_branch,
        base_branch,
        checkout,
    })?;

    if checkout {
        if let Ok(guard) = state.navigation_engine.lock() {
            if let Some(ref engine) = *guard {
                engine.spawn_reconciliation();
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn git_push_vault(state: State<'_, AppState>) -> Result<String, String> {
    let vault_path = {
        let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        match *guard {
            Some(ref p) => p.clone(),
            None => return Err("No hay ninguna bóveda abierta".to_string()),
        }
    };
    let use_case = state.push_versions_use_case.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use_case.execute(PushVersionsCommand { vault_path })
    })
    .await
    .map_err(|e| e.to_string())?
}
