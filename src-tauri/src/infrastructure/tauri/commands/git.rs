use crate::domain::models::git::{
    GitBranchesResult, GitDiffResponse, GitFileLogResult, GitFileRevisionResult, GitImageDiffResult,
    VaultGitStatus,
};
use crate::infrastructure::tauri::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn get_vault_git_status(
    state: State<'_, AppState>,
    folder_path: Option<String>,
) -> Result<VaultGitStatus, String> {
    let vault_path = match folder_path {
        Some(p) => PathBuf::from(p),
        None => {
            let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
            match *guard {
                Some(ref p) => p.clone(),
                None => {
                    return Ok(VaultGitStatus {
                        is_repo: false,
                        branch: None,
                        statuses: Default::default(),
                    });
                }
            }
        }
    };
    state.git_use_cases.get_status(&vault_path)
}

#[tauri::command]
pub fn git_add_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state.git_use_cases.add_paths(vault_path, &paths).map_err(|err| {
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
    state.git_use_cases.restore_paths(vault_path, &paths).map_err(|err| {
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
    state.git_use_cases.restore_staged_paths(vault_path, &paths).map_err(|err| {
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
    state.git_use_cases.commit_paths(vault_path, &paths, &message).map_err(|err| {
        log::error!("[commands::git_commit_paths] {}", err);
        eprintln!("[commands::git_commit_paths] {}", err);
        err
    })
}

#[tauri::command]
pub fn get_git_file_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<GitDiffResponse, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .git_use_cases
        .get_file_diff(vault_path, &path, staged, revision.as_deref())
        .map_err(|err| {
            log::error!("[commands::get_git_file_diff] {}", err);
            eprintln!("[commands::get_git_file_diff] {}", err);
            err
        })
}

#[tauri::command]
pub fn get_git_file_revision(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<GitFileRevisionResult, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let default_rev = if staged { "staged" } else { "HEAD" };
    let rev = revision.as_deref().unwrap_or(default_rev);
    state
        .git_use_cases
        .get_file_revision(vault_path, &path, rev)
        .map_err(|err| {
            log::error!("[commands::get_git_file_revision] {}", err);
            eprintln!("[commands::get_git_file_revision] {}", err);
            err
        })
}

#[tauri::command]
pub fn get_git_image_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<GitImageDiffResult, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .git_use_cases
        .get_image_diff(vault_path, &path, staged, revision.as_deref())
        .map_err(|err| {
            log::error!("[commands::get_git_image_diff] {}", err);
            eprintln!("[commands::get_git_image_diff] {}", err);
            err
        })
}

#[tauri::command]
pub fn get_git_file_log(
    state: State<'_, AppState>,
    path: String,
    max_count: Option<usize>,
) -> Result<GitFileLogResult, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .git_use_cases
        .get_file_log(vault_path, &path, max_count)
        .map_err(|err| {
            log::error!("[commands::get_git_file_log] {}", err);
            eprintln!("[commands::get_git_file_log] {}", err);
            err
        })
}

#[tauri::command]
pub fn get_git_branches(state: State<'_, AppState>) -> Result<GitBranchesResult, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state.git_use_cases.get_branches(vault_path)
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
    let result = state.git_use_cases.checkout_branch(vault_path, &branch_name)?;

    // Sincronizar el árbol de navegación ya que los archivos cambiaron en disco
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
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
    let result = state
        .git_use_cases
        .create_branch(vault_path, &new_branch, &base_branch, checkout)?;

    if checkout {
        if let Ok(guard) = state.navigation_engine.lock() {
            if let Some(ref engine) = *guard {
                engine.reconcile_sync();
            }
        }
    }

    Ok(result)
}
