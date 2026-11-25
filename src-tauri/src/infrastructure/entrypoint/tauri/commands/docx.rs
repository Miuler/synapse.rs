use super::vault::active_vault;
use crate::application::dto::version::docx_source_query::{DocxSource, DocxSourceQuery};
use crate::domain::docx::DocxDiff;
use crate::infrastructure::config::state::AppState;
use serde::Deserialize;
use tauri::State;

/// Como `DiffSource` del frontend.
#[derive(Deserialize)]
pub struct DocxSourceDto {
    kind: String,
    path: String,
    rev: Option<String>,
}

impl DocxSourceDto {
    fn query(self, state: &AppState) -> Result<DocxSourceQuery, String> {
        let source = match (self.kind.as_str(), self.rev) {
            ("disk", _) => {
                state.check_path(&self.path)?;
                DocxSource::Disk
            }
            ("staged", _) => DocxSource::Staged,
            // `git show` no debe tomar la revisión por una opción
            ("rev", Some(rev)) if !rev.starts_with('-') => DocxSource::Revision(rev),
            _ => return Err("Versión del documento no válida".into()),
        };
        Ok(DocxSourceQuery { path: self.path, source })
    }
}

/// Diferencias entre dos versiones de un documento de Word.
#[tauri::command]
pub async fn docx_diff(state: State<'_, AppState>, before: DocxSourceDto, after: DocxSourceDto) -> Result<DocxDiff, String> {
    let vault = active_vault(&state)?;
    let (before, after) = (before.query(&state)?, after.query(&state)?);
    let use_case = state.diff_docx_use_case.clone();
    tauri::async_runtime::spawn_blocking(move || use_case.execute(&vault, &before, &after))
        .await
        .map_err(|e| e.to_string())?
}

/// Bytes de una versión del documento (vacío si no existe).
#[tauri::command]
pub async fn docx_bytes(state: State<'_, AppState>, source: DocxSourceDto) -> Result<tauri::ipc::Response, String> {
    let vault = active_vault(&state)?;
    let query = source.query(&state)?;
    let use_case = state.diff_docx_use_case.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || use_case.read(&vault, &query))
        .await
        .map_err(|e| e.to_string())??;
    Ok(tauri::ipc::Response::new(bytes.unwrap_or_default()))
}

/// Imagen de una versión del documento, por su huella, como data URL.
#[tauri::command]
pub async fn docx_image(state: State<'_, AppState>, source: DocxSourceDto, sha256: String) -> Result<Option<String>, String> {
    let vault = active_vault(&state)?;
    let query = source.query(&state)?;
    let use_case = state.diff_docx_use_case.clone();
    let image = tauri::async_runtime::spawn_blocking(move || use_case.image(&vault, &query, &sha256))
        .await
        .map_err(|e| e.to_string())??;
    use base64::Engine;
    Ok(image.map(|(mime, bytes)| format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))))
}
