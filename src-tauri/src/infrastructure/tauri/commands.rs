use crate::application::use_cases::note_use_cases::NoteUseCases;
use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::note::Note;
use crate::domain::services::search_service::{SearchResult, SearchService};
use crate::infrastructure::repositories::file_note_repository::FileNoteRepository;
use crate::infrastructure::services::nucleo_search_service::NucleoSearchService;
use crate::infrastructure::services::git_service::{GitService, VaultGitStatus};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub active_vault_path: Mutex<PathBuf>,
    pub file_types: SupportedFileTypes,
    pub note_use_cases: NoteUseCases<FileNoteRepository>,
}

impl AppState {
    pub fn new(
        initial_vault_path: PathBuf,
        file_types: SupportedFileTypes,
        note_use_cases: NoteUseCases<FileNoteRepository>,
    ) -> Self {
        Self {
            active_vault_path: Mutex::new(initial_vault_path),
            file_types,
            note_use_cases,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectVaultFolderResult {
    pub folder_path: String,
    pub notes: Vec<Note>,
}

#[tauri::command]
pub fn get_supported_file_types(state: State<'_, AppState>) -> SupportedFileTypes {
    state.file_types.clone()
}

#[tauri::command]
pub fn get_vault_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    state.note_use_cases.list_notes(&vault_path, &state.file_types.all_extensions())
}

#[tauri::command]
pub fn get_active_vault_path(state: State<'_, AppState>) -> Result<String, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    Ok(vault_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn read_note_content(state: State<'_, AppState>, relative_path: String) -> Result<Note, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    state.note_use_cases.read_note(&vault_path, &relative_path)
}

#[tauri::command]
pub fn save_note_content(
    state: State<'_, AppState>,
    relative_path: String,
    title: String,
    content: String,
    encoding: Option<String>,
) -> Result<(), String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());
    state.note_use_cases.save_note(&vault_path, &relative_path, &title, &content, &enc)
}

#[tauri::command]
pub fn set_active_vault_path(state: State<'_, AppState>, new_path: String) -> Result<(), String> {
    let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let path = PathBuf::from(&new_path);
    if path.is_dir() {
        *vault_path = path;
        Ok(())
    } else if path.is_file() {
        if let Some(parent) = path.parent() {
            *vault_path = parent.to_path_buf();
            Ok(())
        } else {
            *vault_path = path;
            Ok(())
        }
    } else {
        *vault_path = path;
        Ok(())
    }
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
                if p.is_dir() {
                    Some(p.clone())
                } else if p.is_file() {
                    p.parent().map(|parent| parent.to_path_buf())
                } else {
                    None
                }
            })
        });

    if let Some(dir) = starting_path {
        dialog = dialog.set_directory(&dir);
    }

    let folder = dialog.pick_folder().await;

    if let Some(folder_handle) = folder {
        let path = folder_handle.path().to_path_buf();
        let folder_path_str = path.to_string_lossy().to_string();

        let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        *vault_path = path.clone();

        let notes = state.note_use_cases.list_notes(&path, &state.file_types.all_extensions())?;
        Ok(Some(SelectVaultFolderResult {
            folder_path: folder_path_str,
            notes,
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn search_items_command(query: String, items: Vec<String>) -> Vec<SearchResult> {
    let search_service = NucleoSearchService::new();
    search_service.search_items(&query, &items)
}

#[tauri::command]
pub fn search_notes_command(state: State<'_, AppState>, query: String) -> Result<Vec<SearchResult>, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let notes = state.note_use_cases.list_notes(&vault_path, &state.file_types.all_extensions())?;

    let search_service = NucleoSearchService::new();
    Ok(search_service.search_notes(&query, &notes))
}

#[tauri::command]
pub fn toggle_devtools(window: tauri::WebviewWindow) {
    if window.is_devtools_open() {
        window.close_devtools();
    } else {
        window.open_devtools();
    }
}

#[tauri::command]
pub fn get_vault_git_status(
    state: State<'_, AppState>,
    folder_path: Option<String>,
) -> Result<VaultGitStatus, String> {
    let vault_path = match folder_path {
        Some(p) => PathBuf::from(p),
        None => state.active_vault_path.lock().map_err(|e| e.to_string())?.clone(),
    };
    let git_service = GitService::new();
    git_service.get_vault_status(&vault_path)
}

#[tauri::command]
pub fn delete_vault_item(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let clean_rel = relative_path.replace('\\', "/");
    let mut target_path = vault_path.clone();
    for part in clean_rel.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err("Ruta no permitida con '..'".to_string());
        }
        target_path.push(part);
    }

    if target_path == *vault_path {
        return Err("No se puede eliminar la raíz de la bóveda".to_string());
    }

    if !target_path.exists() {
        return Err(format!("El elemento '{}' no existe", relative_path));
    }

    let canonical_vault = vault_path.canonicalize().map_err(|e| e.to_string())?;
    let canonical_target = target_path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical_target.starts_with(&canonical_vault) || canonical_target == canonical_vault {
        return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
    }

    if canonical_target.is_dir() {
        std::fs::remove_dir_all(&canonical_target).map_err(|e| format!("Error al eliminar carpeta: {}", e))?;
    } else if canonical_target.is_file() {
        std::fs::remove_file(&canonical_target).map_err(|e| format!("Error al eliminar archivo: {}", e))?;
    } else {
        return Err("Tipo de elemento no soportado para eliminar".to_string());
    }

    Ok(())
}
