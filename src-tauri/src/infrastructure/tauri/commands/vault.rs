use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::note::Note;
use crate::domain::value_objects::note_path::NoteRelativePath;
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
                        sync_state: Some(item.value().sync_state.as_str().to_string()),
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
                        sync_state: Some(item.value().sync_state.as_str().to_string()),
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
                        sync_state: Some("local".to_string()),
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
            sync_state: None,
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
    let note_res = state.note_use_cases.read_note(vault_path, &relative_path);
    let mut note = match note_res {
        Ok(n) => n,
        Err(err) => {
            // Si el archivo no existe en disco pero está indexado como Ghost en el engine,
            // devolvemos una nota placeholder vacía en modo lectura para visualización.
            if let Ok(guard) = state.navigation_engine.lock() {
                if let Some(ref engine) = *guard {
                    if let Some(meta) = engine.get_note_meta(&relative_path) {
                        if meta.sync_state.as_deref() == Some("ghost") {
                            let note_rel = crate::domain::value_objects::note_path::NoteRelativePath::new(&relative_path)
                                .map_err(|e| e.to_string())?;
                            let abs = vault_path.join(&relative_path).to_string_lossy().to_string();
                            Note {
                                relative_path: note_rel,
                                abs_path: abs,
                                title: meta.basename.clone(),
                                content: String::new(),
                                view_mode: meta.view_mode.clone(),
                                encoding: "UTF-8".to_string(),
                                frontmatter: None,
                            }
                        } else {
                            return Err(err);
                        }
                    } else {
                        return Err(err);
                    }
                } else {
                    return Err(err);
                }
            } else {
                return Err(err);
            }
        }
    };
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.record_opened(&relative_path);
            note.view_mode = engine.get_note_view_mode(&relative_path);
        }
    }
    Ok(note)
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
