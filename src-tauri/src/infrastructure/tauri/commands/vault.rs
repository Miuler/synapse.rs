use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::note::Note;
use crate::domain::value_objects::note_path::NoteRelativePath;
use crate::infrastructure::navigation::engine::{OpenTabDto, VaultUiState, WorkspaceOpenTabsState};
use crate::infrastructure::tauri::dto::{VaultEntryNode, VaultFileDto, VaultFilesResponse};
use crate::infrastructure::tauri::state::AppState;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_supported_file_types(state: State<'_, AppState>) -> SupportedFileTypes {
    state.file_types.clone()
}

#[tauri::command]
pub fn get_vault_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let engine_guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let (Some(vault_path), Some(engine)) = (vault_guard.as_ref(), engine_guard.as_ref()) else {
        return Ok(Vec::new());
    };

    let mut notes = Vec::with_capacity(engine.notes.len());
    for item in engine.notes.iter() {
        let meta = item.value();
        let rel_str = meta.path.to_string();
        let abs_path = vault_path.join(&rel_str).to_string_lossy().to_string();
        if let Ok(rel_path) = NoteRelativePath::new(&rel_str) {
            let mut note = Note::new(
                rel_path,
                abs_path,
                meta.title.to_string(),
                String::new(), // Zero content kept in memory for list metadata
            );
            note.view_mode = meta.view_mode.as_ref().map(|s| s.to_string());
            notes.push(note);
        }
    }
    notes.sort_by(|a, b| a.relative_path.as_str().cmp(b.relative_path.as_str()));
    Ok(notes)
}

#[tauri::command]
pub fn get_vault_files(state: State<'_, AppState>) -> Result<VaultFilesResponse, String> {
    let engine_guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    Ok(VaultFilesResponse {
        engine: engine_guard.as_ref().map(Arc::clone),
    })
}

#[tauri::command]
pub fn get_note_meta(
    relative_path: String,
    state: State<'_, AppState>,
) -> Result<Option<VaultFileDto>, String> {
    let engine_guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(engine) = engine_guard.as_ref() else {
        return Ok(None);
    };

    Ok(engine.get_note_meta(&relative_path))
}

#[tauri::command]
pub fn get_vault_directory_children(
    state: State<'_, AppState>,
    parent_path: Option<String>,
) -> Result<Vec<VaultEntryNode>, String> {
    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let engine_guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let (Some(vault_path), Some(engine)) = (vault_guard.as_ref(), engine_guard.as_ref()) else {
        return Ok(Vec::new());
    };

    let clean_parent = parent_path
        .map(|p| p.trim_matches('/').replace('\\', "/"))
        .unwrap_or_default();

    let mut folders: BTreeSet<String> = BTreeSet::new();
    let mut files: BTreeMap<String, VaultEntryNode> = BTreeMap::new();

    let prefix = if clean_parent.is_empty() {
        String::new()
    } else {
        format!("{}/", clean_parent)
    };

    for item in engine.notes.iter() {
        let rel_path = item.value().path.as_str();
        if prefix.is_empty() {
            if let Some((folder, _)) = rel_path.split_once('/') {
                folders.insert(folder.to_string());
            } else {
                files.insert(
                    rel_path.to_string(),
                    VaultEntryNode {
                        name: rel_path.to_string(),
                        relative_path: rel_path.to_string(),
                        is_folder: false,
                        title: Some(item.value().title.to_string()),
                    },
                );
            }
        } else if let Some(sub) = rel_path.strip_prefix(&prefix) {
            if let Some((folder, _)) = sub.split_once('/') {
                let full_folder_rel = format!("{}/{}", clean_parent, folder);
                folders.insert(full_folder_rel);
            } else {
                files.insert(
                    rel_path.to_string(),
                    VaultEntryNode {
                        name: sub.to_string(),
                        relative_path: rel_path.to_string(),
                        is_folder: false,
                        title: Some(item.value().title.to_string()),
                    },
                );
            }
        }
    }

    // Inspect the immediate directory on disk to include empty directories
    let disk_dir = if clean_parent.is_empty() {
        vault_path.clone()
    } else {
        vault_path.join(&clean_parent)
    };

    if let Ok(entries) = std::fs::read_dir(&disk_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if crate::infrastructure::navigation::watcher::is_ignored_dir_or_file(&file_name) {
                continue; // Skip hidden dirs and build/dependency folders
            }
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let full_rel = if clean_parent.is_empty() {
                        file_name
                    } else {
                        format!("{}/{}", clean_parent, file_name)
                    };
                    folders.insert(full_rel);
                } else if ft.is_file() && state.file_types.is_supported_file(&file_name) {
                    let full_rel = if clean_parent.is_empty() {
                        file_name.clone()
                    } else {
                        format!("{}/{}", clean_parent, file_name)
                    };
                    files.entry(full_rel.clone()).or_insert_with(|| VaultEntryNode {
                        name: file_name,
                        relative_path: full_rel,
                        is_folder: false,
                        title: None,
                    });
                }
            }
        }
    }

    let mut result = Vec::new();
    for folder_rel in folders {
        let name = folder_rel.split('/').next_back().unwrap_or(&folder_rel).to_string();
        result.push(VaultEntryNode {
            name,
            relative_path: folder_rel,
            is_folder: true,
            title: None,
        });
    }

    for (_, file_node) in files {
        result.push(file_node);
    }

    Ok(result)
}

#[tauri::command]
pub fn get_active_vault_path(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    Ok(vault_path.as_ref().map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn read_note_content(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<Note, String> {
    let p = Path::new(&relative_path);
    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;

    // Modo Archivo Puntual: si la ruta es absoluta o no hay ninguna bóveda activa
    if p.is_absolute() || vault_guard.is_none() {
        return state.note_use_cases.read_standalone_note(p);
    }

    let Some(ref vault_path) = *vault_guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let mut note = state.note_use_cases.read_note(vault_path, &relative_path)?;
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.record_opened(&relative_path);
            note.view_mode = engine.get_note_view_mode(&relative_path);
        }
    }
    Ok(note)
}

#[tauri::command]
pub fn record_note_opened(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.record_opened(&relative_path);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn save_note_content(
    state: State<'_, AppState>,
    relative_path: String,
    title: String,
    content: String,
    encoding: Option<String>,
) -> Result<(), String> {
    let p = Path::new(&relative_path);
    let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());
    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;

    // Modo Archivo Puntual: si la ruta es absoluta o no hay ninguna bóveda activa
    if p.is_absolute() || vault_guard.is_none() {
        return state.note_use_cases.save_standalone_note(p, &title, &content, &enc);
    }

    let Some(ref vault_path) = *vault_guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .note_use_cases
        .save_note(vault_path, &relative_path, &title, &content, &enc)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_active_vault_path(state: State<'_, AppState>, new_path: String) -> Result<(), String> {
    let path = PathBuf::from(&new_path);
    let target_dir = if path.is_dir() {
        path
    } else if path.is_file() {
        path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
    } else {
        path
    };

    if !target_dir.exists() || !target_dir.is_dir() {
        let _ = state.workspace_registry_use_cases.remove_workspace(&new_path);
        return Err("Ruta de bóveda inválida o no existe".to_string());
    }

    let synapse_dir = target_dir.join(".synapse");
    if !synapse_dir.exists() {
        let confirmed = rfd::AsyncMessageDialog::new()
            .set_title("Advertencia de Bóveda")
            .set_description(format!(
                "El directorio seleccionado:\n{}\n\nNo contiene una carpeta '.synapse'.\n¿Deseas abrirlo como una bóveda? Ten en cuenta que se indexarán todos los archivos y notas contenidos en él.",
                target_dir.display()
            ))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            .await;

        if confirmed != rfd::MessageDialogResult::Yes {
            return Err("Apertura de bóveda cancelada por el usuario".to_string());
        }

        if let Err(e) = std::fs::create_dir_all(&synapse_dir) {
            return Err(format!("No se pudo crear el directorio .synapse: {}", e));
        }
    }

    let _ = state.activate_vault(target_dir, None)?;
    Ok(())
}

#[tauri::command]
pub fn get_vault_files_count(state: State<'_, AppState>) -> Result<usize, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(0);
    };
    Ok(engine.notes.len())
}

#[tauri::command]
pub fn get_recent_notes_command(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(Vec::new());
    };
    let recent = engine.get_recent_notes(limit.unwrap_or(15));
    Ok(recent.into_iter().map(|n| n.path.to_string()).collect())
}

#[tauri::command]
pub fn save_open_tabs_state(
    state: State<'_, AppState>,
    tabs: Vec<OpenTabDto>,
    active_tab: Option<String>,
) -> Result<(), String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *guard {
        engine.save_open_tabs_state(&tabs, active_tab.as_deref());
    }
    Ok(())
}

#[tauri::command]
pub fn get_open_tabs_state(
    state: State<'_, AppState>,
) -> Result<WorkspaceOpenTabsState, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(WorkspaceOpenTabsState {
            open_tabs: Vec::new(),
            active_tab: None,
        });
    };
    Ok(engine.get_open_tabs_state())
}

#[tauri::command]
pub fn set_note_view_mode(
    state: State<'_, AppState>,
    relative_path: String,
    view_mode: String,
) -> Result<(), String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *guard {
        engine.set_note_view_mode(&relative_path, &view_mode);
    }
    Ok(())
}

#[tauri::command]
pub fn save_vault_ui_state(
    state: State<'_, AppState>,
    sidebar_width: Option<u32>,
    expanded_folders: Option<Vec<String>>,
    diff_visual_mode: Option<String>,
    diff_text_mode: Option<String>,
    locked_files: Option<Vec<String>>,
) -> Result<(), String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *guard {
        engine.save_vault_ui_state(
            sidebar_width,
            expanded_folders,
            diff_visual_mode,
            diff_text_mode,
            locked_files,
        );
    }
    Ok(())
}

#[tauri::command]
pub fn get_vault_ui_state(
    state: State<'_, AppState>,
) -> Result<VaultUiState, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(VaultUiState::default());
    };
    Ok(engine.get_vault_ui_state())
}

#[tauri::command]
pub fn delete_vault_item(state: State<'_, AppState>, relative_path: String) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state.file_system_use_cases.delete_item(vault_path, &relative_path)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn rename_vault_item(
    state: State<'_, AppState>,
    relative_path: String,
    new_name: String,
) -> Result<String, String> {
    let vault_path = {
        let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        match *guard {
            Some(ref path) => path.clone(),
            None => return Err("No hay ninguna bóveda abierta".to_string()),
        }
    };

    let renamed_path = state.file_system_use_cases.rename_item(&vault_path, &relative_path, &new_name)?;
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }
    Ok(renamed_path)
}

/// Pega (copia) los elementos indicados dentro de `dest_dir`.
/// Retorna las rutas relativas de los nuevos elementos creados.
#[tauri::command]
pub fn copy_vault_items(
    state: State<'_, AppState>,
    paths: Vec<String>,
    dest_dir: String,
) -> Result<Vec<String>, String> {
    let vault_path = {
        let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        match *guard {
            Some(ref p) => p.clone(),
            None => return Err("No hay ninguna bóveda abierta".to_string()),
        }
    };

    let created = state.file_system_use_cases.copy_items(&vault_path, &paths, &dest_dir)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            // Indexa sólo lo creado (incluye contenido de carpetas) y notifica al frontend
            let _ = engine.reload_paths(&created);
        }
    }

    Ok(created)
}

#[tauri::command]
pub fn reload_vault_items(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let guard = state
        .navigation_engine
        .lock()
        .map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(Vec::new());
    };
    engine.reload_paths(&paths)
}

pub fn percent_decode_str(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let mut chars = s.as_bytes().iter().copied();
    while let Some(b) = chars.next() {
        if b == b'%' {
            if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                let v1 = match h1 {
                    b'0'..=b'9' => Some(h1 - b'0'),
                    b'a'..=b'f' => Some(h1 - b'a' + 10),
                    b'A'..=b'F' => Some(h1 - b'A' + 10),
                    _ => None,
                };
                let v2 = match h2 {
                    b'0'..=b'9' => Some(h2 - b'0'),
                    b'a'..=b'f' => Some(h2 - b'a' + 10),
                    b'A'..=b'F' => Some(h2 - b'A' + 10),
                    _ => None,
                };
                if let (Some(val1), Some(val2)) = (v1, v2) {
                    bytes.push((val1 << 4) | val2);
                    continue;
                }
                bytes.push(b'%');
                bytes.push(h1);
                bytes.push(h2);
            } else {
                bytes.push(b'%');
            }
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| s.to_string())
}

pub fn resolve_asset_file_path_internal(
    state: &AppState,
    src: &str,
    base_file: Option<&str>,
) -> Result<Option<String>, String> {
    let raw = src.trim().trim_matches('\'').trim_matches('"');
    if raw.is_empty() {
        return Ok(None);
    }

    // 1. Limpiar posibles wrappers WikiLink: ![[...]] o [[...]]
    let cleaned = raw
        .strip_prefix("![[")
        .and_then(|s| s.strip_suffix("]]"))
        .or_else(|| raw.strip_prefix("[[").and_then(|s| s.strip_suffix("]]")))
        .unwrap_or(raw)
        .trim();

    // 2. Separar query params (?t=...) y anclas (#...)
    let without_query = cleaned
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("")
        .trim();
    if without_query.is_empty() {
        return Ok(None);
    }

    // 3. Manejar esquemas asset:// o asset://localhost
    let mut path_candidate = without_query;
    if let Some(stripped) = path_candidate.strip_prefix("asset://localhost") {
        path_candidate = stripped;
    } else if let Some(stripped) = path_candidate.strip_prefix("asset://") {
        path_candidate = stripped;
    }

    // 4. Decodificar caracteres URL-encoded (ej: %20 para espacios, %40 para @, %F0%9F... para emojis)
    let decoded = percent_decode_str(path_candidate);
    #[allow(unused_mut)]
    let mut check_str = decoded.as_str();

    // En Unix, si venía de un esquema asset o ruta que perdió la barra inicial
    #[cfg(unix)]
    let linux_abs_buf: String;
    #[cfg(unix)]
    if !check_str.starts_with('/') && (check_str.starts_with("home/") || check_str.starts_with("tmp/") || check_str.starts_with("var/") || check_str.starts_with("usr/")) {
        linux_abs_buf = format!("/{}", check_str);
        check_str = &linux_abs_buf;
    }

    // 5. Si ya es una ruta absoluta en el sistema y es un archivo existente
    let path_buf = PathBuf::from(check_str);
    if path_buf.is_absolute() && path_buf.is_file() {
        return Ok(Some(path_buf.to_string_lossy().to_string()));
    }

    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *vault_guard else {
        return Ok(None);
    };

    // 6. Intentar buscar en el DashMap (resolución O(1) e indexada de WikiLinks y subdirectorios con proximidad)
    let engine_guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *engine_guard {
        if let Some(resolved_rel) = engine.resolve_link_path_with_base(check_str, base_file) {
            let (rel_without_anchor, _) = crate::domain::services::link_resolution::normalize_link_target(&resolved_rel);
            let abs = vault_path.join(&rel_without_anchor);
            if abs.is_file() {
                return Ok(Some(abs.to_string_lossy().to_string()));
            }
        }
    }

    // 7. Intentar ruta relativa directa a la raíz de la bóveda
    let clean_rel = check_str.trim_start_matches("./").trim_start_matches('/');
    let direct_abs = vault_path.join(clean_rel);
    if direct_abs.is_file() {
        return Ok(Some(direct_abs.to_string_lossy().to_string()));
    }

    // 8. Intentar ruta relativa respecto al archivo actual (base_file)
    if let Some(base) = base_file {
        let base_clean = base.trim_start_matches("./").trim_start_matches('/');
        if let Some(parent) = Path::new(base_clean).parent() {
            let relative_abs = vault_path.join(parent).join(clean_rel);
            if relative_abs.is_file() {
                return Ok(Some(relative_abs.to_string_lossy().to_string()));
            }
        }
    }

    Ok(None)
}

#[tauri::command]
pub fn resolve_asset_file_path(
    state: State<'_, AppState>,
    src: String,
    base_file: Option<String>,
) -> Result<Option<String>, String> {
    match resolve_asset_file_path_internal(&state, &src, base_file.as_deref()) {
        Ok(Some(path)) => Ok(Some(path)),
        Ok(None) => {
            log::warn!("No se pudo resolver el asset: src={:?}, base_file={:?}", src, base_file);
            Ok(None)
        }
        Err(error) => {
            log::error!("Error resolviendo asset: src={:?}, base_file={:?}, error={}", src, base_file, error);
            Err(error)
        }
    }
}

pub fn get_image_mime_type(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "tiff" | "tif" => "image/tiff",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[tauri::command]
pub fn save_binary_asset(
    state: State<'_, AppState>,
    relative_path: String,
    base64_data: String,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;

    let raw_b64 = if let Some(idx) = base64_data.find(";base64,") {
        &base64_data[idx + 8..]
    } else {
        &base64_data
    };

    let bytes = BASE64_STANDARD
        .decode(raw_b64.trim())
        .map_err(|e| format!("Error decodificando base64: {}", e))?;

    state
        .file_system_use_cases
        .write_binary_item(vault_path, &relative_path, &bytes)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn read_binary_file(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<Vec<u8>, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state.file_system_use_cases.read_binary_item(vault_path, &relative_path)
}

#[tauri::command]
pub fn write_binary_file(
    state: State<'_, AppState>,
    relative_path: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state
        .file_system_use_cases
        .write_binary_item(vault_path, &relative_path, &data)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn create_vault_folder(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state
        .file_system_use_cases
        .create_folder(vault_path, &relative_path)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn read_asset_data_url(
    state: State<'_, AppState>,
    src: String,
    base_file: Option<String>,
) -> Result<Option<String>, String> {
    let resolved = match resolve_asset_file_path_internal(&state, &src, base_file.as_deref()) {
        Ok(resolved) => resolved,
        Err(error) => {
            log::error!("Error resolviendo asset para Data URL: src={:?}, base_file={:?}, error={}", src, base_file, error);
            return Err(error);
        }
    };
    let Some(abs_path) = resolved else {
        log::warn!("No se pudo leer asset como Data URL: src={:?}, base_file={:?}", src, base_file);
        return Ok(None);
    };

    let path = Path::new(&abs_path);
    if !path.is_file() {
        log::warn!("El asset resuelto no es un archivo: path={:?}", abs_path);
        return Ok(None);
    }

    let bytes = std::fs::read(path).map_err(|e| {
        log::error!("Error leyendo asset para Data URL: path={:?}, error={}", abs_path, e);
        format!("Error leyendo {}: {}", abs_path, e)
    })?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mime = get_image_mime_type(ext);

    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;
    let b64 = BASE64_STANDARD.encode(&bytes);
    Ok(Some(format!("data:{};base64,{}", mime, b64)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::navigation::engine::NavigationEngine;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_percent_decode_str() {
        assert_eq!(percent_decode_str("simple"), "simple");
        assert_eq!(percent_decode_str("hello%20world"), "hello world");
        assert_eq!(percent_decode_str("user%40example.com"), "user@example.com");
        // UTF-8 bytes for 👷: 0xF0, 0x9F, 0x91, 0xB7
        assert_eq!(percent_decode_str("%F0%9F%91%B7%20Trabajo"), "👷 Trabajo");
    }

    #[test]
    fn test_resolve_asset_file_path_and_data_url() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().to_path_buf();

        let sub_dir = vault_path.join("👷 Trabajo").join("adjuntos");
        std::fs::create_dir_all(&sub_dir).unwrap();
        let img_path = sub_dir.join("photo test.png");
        {
            let mut f = File::create(&img_path).unwrap();
            f.write_all(b"\x89PNG\r\n\x1a\nfakeimagebytes").unwrap();
        }

        let file_types = SupportedFileTypes::default();
        let repo = crate::infrastructure::repositories::file_note_repository::FileNoteRepository::new();
        let use_cases = crate::application::use_cases::note_use_cases::NoteUseCases::new(repo);
        let state = AppState::empty(file_types, use_cases);

        *state.active_vault_path.lock().unwrap() = Some(vault_path.clone());
        let engine = Arc::new(NavigationEngine::new(
            vault_path.clone(),
            vault_path.join(".synapse/cache.bin"),
        ));
        engine.reconcile_sync();
        *state.navigation_engine.lock().unwrap() = Some(engine);

        // 1. Direct absolute path
        let abs_res = resolve_asset_file_path_internal(&state, img_path.to_str().unwrap(), None).unwrap();
        assert_eq!(abs_res, Some(img_path.to_string_lossy().to_string()));

        // 2. Asset URL with encoded spaces, emoji and query param ?t=123
        let asset_url = format!("asset://localhost{}?t=123", img_path.to_string_lossy())
            .replace(' ', "%20")
            .replace("👷", "%F0%9F%91%B7");
        let asset_res = resolve_asset_file_path_internal(&state, &asset_url, None).unwrap();
        assert_eq!(asset_res, Some(img_path.to_string_lossy().to_string()));

        // 3. Vault relative path
        let rel_res = resolve_asset_file_path_internal(
            &state,
            "👷 Trabajo/adjuntos/photo test.png",
            None,
        ).unwrap();
        assert_eq!(rel_res, Some(img_path.to_string_lossy().to_string()));

        // 4. WikiLink lookup via DashMap: "photo test.png"
        let wiki_res = resolve_asset_file_path_internal(&state, "[[photo test.png]]", None).unwrap();
        assert_eq!(wiki_res, Some(img_path.to_string_lossy().to_string()));

        // 5. Embed WikiLink: "![[photo test.png]]"
        let embed_res = resolve_asset_file_path_internal(&state, "![[photo test.png]]", None).unwrap();
        assert_eq!(embed_res, Some(img_path.to_string_lossy().to_string()));

        // 6. Test reading Data URL directly from WikiLink
        let resolved = resolve_asset_file_path_internal(&state, "[[photo test.png]]", None).unwrap().unwrap();
        let bytes = std::fs::read(&resolved).unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(get_image_mime_type("png"), "image/png");
    }
}
