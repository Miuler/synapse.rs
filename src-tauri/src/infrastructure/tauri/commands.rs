use crate::application::use_cases::note_use_cases::NoteUseCases;
use crate::domain::models::note::Note;
use crate::domain::services::search_service::{SearchResult, SearchService};
use crate::infrastructure::repositories::file_note_repository::FileNoteRepository;
use crate::infrastructure::services::nucleo_search_service::NucleoSearchService;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub active_vault_path: Mutex<PathBuf>,
    pub supported_extensions: Vec<String>,
    pub note_use_cases: NoteUseCases<FileNoteRepository>,
}

impl AppState {
    pub fn new(
        initial_vault_path: PathBuf,
        supported_extensions: Vec<String>,
        note_use_cases: NoteUseCases<FileNoteRepository>,
    ) -> Self {
        Self {
            active_vault_path: Mutex::new(initial_vault_path),
            supported_extensions,
            note_use_cases,
        }
    }
}

#[tauri::command]
pub fn get_vault_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    state.note_use_cases.list_notes(&vault_path, &state.supported_extensions)
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
) -> Result<(), String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    state.note_use_cases.save_note(&vault_path, &relative_path, &title, &content)
}

#[tauri::command]
pub fn set_active_vault_path(state: State<'_, AppState>, new_path: String) -> Result<(), String> {
    let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    *vault_path = PathBuf::from(new_path);
    Ok(())
}

#[tauri::command]
pub async fn select_vault_folder(state: State<'_, AppState>) -> Result<Option<Vec<Note>>, String> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Seleccionar Carpeta / Bóveda")
        .pick_folder()
        .await;

    if let Some(folder_handle) = folder {
        let path = folder_handle.path().to_path_buf();
        let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        *vault_path = path.clone();
        
        let notes = state.note_use_cases.list_notes(&path, &state.supported_extensions)?;
        Ok(Some(notes))
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
    let notes = state.note_use_cases.list_notes(&vault_path, &state.supported_extensions)?;

    let search_service = NucleoSearchService::new();
    Ok(search_service.search_notes(&query, &notes))
}
