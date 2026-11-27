use crate::application::use_cases::board::read_board_use_case::ReadBoardUseCase;
use crate::application::use_cases::board::save_board_use_case::SaveBoardUseCase;
use crate::domain::board::board_error::{BoardError, BoardErrorKind};
use crate::domain::board::board_file::BoardFile;
use crate::domain::board::board_repository::{BoardSnapshot, StoredTask};
use crate::domain::board::task_file::TaskFile;
use crate::domain::note::note_path::NoteRelativePath;
use crate::infrastructure::config::state::AppState;
use crate::infrastructure::repository::file::board_repository_impl::BoardRepositoryImpl;
use std::path::PathBuf;
use tauri::State;

/// La ruta absoluta de un tablero de la bóveda abierta.
fn resolve(state: &State<'_, AppState>, path: &str) -> Result<PathBuf, BoardError> {
    let bad = |detail: String| BoardError::new(BoardErrorKind::Path, detail);
    let relative = NoteRelativePath::new(path).map_err(bad)?;
    let vault = state
        .active_vault_path
        .lock()
        .map_err(|e| bad(e.to_string()))?
        .clone()
        .ok_or_else(|| bad("No hay ninguna bóveda abierta".into()))?;
    Ok(vault.join(relative.as_str()))
}

/// Los tableros grandes leen miles de archivos: fuera del hilo asíncrono.
async fn disk<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, BoardError> + Send + 'static,
) -> Result<T, BoardError> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| BoardError::new(BoardErrorKind::Io, e.to_string()))?
}

#[tauri::command]
pub async fn board_read(
    state: State<'_, AppState>,
    path: String,
) -> Result<BoardSnapshot, BoardError> {
    let board = resolve(&state, &path)?;
    disk(move || ReadBoardUseCase::new(BoardRepositoryImpl).execute(&board)).await
}

#[tauri::command]
pub async fn board_write(
    state: State<'_, AppState>,
    path: String,
    board: BoardFile,
    hash: Option<String>,
) -> Result<String, BoardError> {
    let path = resolve(&state, &path)?;
    disk(move || SaveBoardUseCase::new(BoardRepositoryImpl).board(&path, &board, hash.as_deref()))
        .await
}

#[tauri::command]
pub async fn board_task_read(
    state: State<'_, AppState>,
    path: String,
    id: String,
) -> Result<StoredTask, BoardError> {
    let board = resolve(&state, &path)?;
    disk(move || ReadBoardUseCase::new(BoardRepositoryImpl).task(&board, &id)).await
}

#[tauri::command]
pub async fn board_task_write(
    state: State<'_, AppState>,
    path: String,
    id: String,
    task: TaskFile,
    hash: Option<String>,
) -> Result<String, BoardError> {
    let board = resolve(&state, &path)?;
    disk(move || {
        SaveBoardUseCase::new(BoardRepositoryImpl).task(&board, &id, &task, hash.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn board_task_delete(
    state: State<'_, AppState>,
    path: String,
    id: String,
    hash: String,
) -> Result<(), BoardError> {
    let board = resolve(&state, &path)?;
    disk(move || SaveBoardUseCase::new(BoardRepositoryImpl).delete_task(&board, &id, &hash)).await
}
