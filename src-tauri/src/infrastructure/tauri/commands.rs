use crate::application::use_cases::note_use_cases::NoteUseCases;
use crate::domain::models::note::Note;
use crate::infrastructure::repositories::file_note_repository::FileNoteRepository;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub active_vault_path: Mutex<PathBuf>,
}

impl AppState {
    pub fn new(initial_vault_path: PathBuf) -> Self {
        Self {
            active_vault_path: Mutex::new(initial_vault_path),
        }
    }
}

#[tauri::command]
pub fn get_vault_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let repo = FileNoteRepository::new();
    let use_cases = NoteUseCases::new(repo);
    use_cases.list_notes(&vault_path)
}

#[tauri::command]
pub fn read_note_content(state: State<'_, AppState>, relative_path: String) -> Result<Note, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let repo = FileNoteRepository::new();
    let use_cases = NoteUseCases::new(repo);
    use_cases.read_note(&vault_path, &relative_path)
}

#[tauri::command]
pub fn save_note_content(
    state: State<'_, AppState>,
    relative_path: String,
    title: String,
    content: String,
) -> Result<(), String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let repo = FileNoteRepository::new();
    let use_cases = NoteUseCases::new(repo);
    use_cases.save_note(&vault_path, &relative_path, &title, &content)
}

#[tauri::command]
pub fn set_active_vault_path(state: State<'_, AppState>, new_path: String) -> Result<(), String> {
    let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    *vault_path = PathBuf::from(new_path);
    Ok(())
}
