use crate::domain::models::config::ShortcutsConfig;
use crate::domain::models::workspace_registry::RecentWorkspaceEntry;
use crate::infrastructure::tauri::dto::SelectVaultFolderResult;
use crate::infrastructure::tauri::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultDirectoryList {
    pub files: Vec<String>,
    pub folders: Vec<String>,
}

#[tauri::command]
pub async fn select_vault_folder(
    state: State<'_, AppState>,
    starting_directory: Option<String>,
) -> Result<Option<SelectVaultFolderResult>, String> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title("Seleccionar Carpeta / Bóveda");

    let starting_path = starting_directory
        .as_deref()
        .map(PathBuf::from)
        .and_then(|p| {
            if p.is_dir() {
                Some(p)
            } else if p.is_file() {
                p.parent().map(|parent| parent.to_path_buf())
            } else {
                None
            }
        })
        .or_else(|| {
            state.active_vault_path.lock().ok().and_then(|p| {
                p.as_ref().and_then(|path| {
                    if path.is_dir() {
                        Some(path.clone())
                    } else if path.is_file() {
                        path.parent().map(|parent| parent.to_path_buf())
                    } else {
                        None
                    }
                })
            })
        });

    if let Some(dir) = starting_path {
        dialog = dialog.set_directory(&dir);
    }

    let folder = dialog.pick_folder().await;

    if let Some(folder_handle) = folder {
        let path = folder_handle.path().to_path_buf();

        let synapse_dir = path.join(".synapse");
        if !synapse_dir.exists() {
            let confirmed = rfd::AsyncMessageDialog::new()
                .set_title("Advertencia de Bóveda")
                .set_description(format!(
                    "El directorio seleccionado:\n{}\n\nNo contiene una carpeta '.synapse'.\n¿Deseas abrirlo como una bóveda? Ten en cuenta que se indexarán todos los archivos y notas contenidos en él.",
                    path.display()
                ))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                .await;

            if confirmed != rfd::MessageDialogResult::Yes {
                return Ok(None);
            }

            if let Err(e) = std::fs::create_dir_all(&synapse_dir) {
                return Err(format!("No se pudo crear el directorio .synapse: {}", e));
            }
        }

        let res = state.activate_vault(path, None)?;
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn get_recent_workspaces(state: State<'_, AppState>) -> Result<Vec<RecentWorkspaceEntry>, String> {
    state.workspace_registry_use_cases.list_workspaces()
}

#[tauri::command]
pub fn get_shortcuts_config(state: State<'_, AppState>) -> Result<ShortcutsConfig, String> {
    state.workspace_registry_use_cases.get_shortcuts_config()
}

#[tauri::command]
pub fn toggle_pin_workspace(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<RecentWorkspaceEntry>, String> {
    state.workspace_registry_use_cases.toggle_pin(&path)
}

#[tauri::command]
pub fn remove_recent_workspace(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<RecentWorkspaceEntry>, String> {
    state.workspace_registry_use_cases.remove_workspace(&path)
}

#[tauri::command]
pub async fn create_new_workspace(
    state: State<'_, AppState>,
    starting_directory: Option<String>,
) -> Result<Option<SelectVaultFolderResult>, String> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title("Seleccionar o Crear Directorio para Nuevo Workspace");

    if let Some(dir) = starting_directory.as_deref().map(PathBuf::from).filter(|p| p.is_dir()) {
        dialog = dialog.set_directory(&dir);
    } else if let Ok(home) = std::env::var("HOME") {
        let hp = PathBuf::from(home);
        if hp.is_dir() {
            dialog = dialog.set_directory(&hp);
        }
    }

    let folder = dialog.pick_folder().await;

    if let Some(folder_handle) = folder {
        let path = folder_handle.path().to_path_buf();

        let synapse_dir = path.join(".synapse");
        if let Err(e) = std::fs::create_dir_all(&synapse_dir) {
            return Err(format!("No se pudo crear el directorio .synapse: {}", e));
        }

        let res = state.activate_vault(path, None)?;
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn clone_git_workspace(
    state: State<'_, AppState>,
    repo_url: String,
    starting_directory: Option<String>,
) -> Result<Option<SelectVaultFolderResult>, String> {
    let trimmed_url = repo_url.trim().to_string();
    if trimmed_url.is_empty() {
        return Err("La URL del repositorio Git no puede estar vacía".to_string());
    }

    let mut dialog = rfd::AsyncFileDialog::new().set_title("Seleccionar Carpeta Destino para Clonar Repositorio");

    if let Some(dir) = starting_directory.as_deref().map(PathBuf::from).filter(|p| p.is_dir()) {
        dialog = dialog.set_directory(&dir);
    } else if let Ok(home) = std::env::var("HOME") {
        let hp = PathBuf::from(home);
        if hp.is_dir() {
            dialog = dialog.set_directory(&hp);
        }
    }

    let Some(folder_handle) = dialog.pick_folder().await else {
        return Ok(None);
    };

    let parent_dir = folder_handle.path().to_path_buf();
    let repo_name = trimmed_url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit(['/', ':'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("synapse-vault");

    let target_dir = parent_dir.join(repo_name);

    state
        .git_use_cases
        .clone_repository(&trimmed_url, &target_dir)?;

    let synapse_dir = target_dir.join(".synapse");
    let _ = std::fs::create_dir_all(&synapse_dir);

    let res = state.activate_vault(target_dir, Some(true))?;
    Ok(Some(res))
}

#[tauri::command]
pub fn open_workspace_in_new_window(
    app: tauri::AppHandle,
    path: String,
) -> Result<(), String> {
    let target_dir = PathBuf::from(&path);
    if !target_dir.is_dir() {
        return Err("El directorio del workspace no existe".to_string());
    }
    let name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Workspace");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let window_label = format!("ws_win_{}", timestamp);

    let encoded_path = path.replace('\\', "/");
    let url = format!("index.html?vault={}", encoded_path);

    tauri::WebviewWindowBuilder::new(
        &app,
        &window_label,
        tauri::WebviewUrl::App(PathBuf::from(url)),
    )
    .title(format!("Synapse - {}", name))
    .inner_size(1100.0, 760.0)
    .decorations(false)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("Error creando nueva ventana de workspace: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn check_vault_path_exists(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<bool, String> {
    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *vault_guard else {
        return Ok(false);
    };
    let clean = relative_path.trim_matches('/').replace('\\', "/");
    if clean.is_empty() {
        return Ok(true);
    }
    let p = vault_path.join(clean);
    Ok(p.exists())
}

#[tauri::command]
pub fn list_vault_directory(
    state: State<'_, AppState>,
    relative_path: Option<String>,
) -> Result<VaultDirectoryList, String> {
    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *vault_guard else {
        return Ok(VaultDirectoryList {
            files: Vec::new(),
            folders: Vec::new(),
        });
    };

    let clean = relative_path
        .map(|p| p.trim_matches('/').replace('\\', "/"))
        .unwrap_or_default();
    let target_dir = if clean.is_empty() {
        vault_path.clone()
    } else {
        vault_path.join(&clean)
    };

    let mut files = Vec::new();
    let mut folders = Vec::new();

    if target_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let full_rel = if clean.is_empty() {
                    file_name.clone()
                } else {
                    format!("{}/{}", clean, file_name)
                };

                if let Ok(ft) = entry.file_type() {
                    if ft.is_dir() {
                        folders.push(full_rel);
                    } else if ft.is_file() {
                        files.push(full_rel);
                    }
                }
            }
        }
    }

    Ok(VaultDirectoryList { files, folders })
}
