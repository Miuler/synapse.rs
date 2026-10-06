use crate::application::use_cases::note_use_cases::NoteUseCases;
use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::note::Note;
use crate::domain::services::search_service::{SearchResult, SearchService};
use crate::domain::value_objects::note_path::NoteRelativePath;
use crate::infrastructure::repositories::file_note_repository::FileNoteRepository;
use crate::infrastructure::services::git_service::{GitService, VaultGitStatus};
use crate::infrastructure::services::nucleo_search_service::NucleoSearchService;
use crate::navigation::engine::NavigationEngine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    pub active_vault_path: Mutex<PathBuf>,
    pub file_types: SupportedFileTypes,
    pub note_use_cases: NoteUseCases<FileNoteRepository>,
    pub navigation_engine: Mutex<Arc<NavigationEngine>>,
}

impl AppState {
    pub fn new(
        initial_vault_path: PathBuf,
        file_types: SupportedFileTypes,
        note_use_cases: NoteUseCases<FileNoteRepository>,
    ) -> Self {
        let cache_path = initial_vault_path.join(".synapse").join("cache.bin");
        let engine = NavigationEngine::start_with_file_types(
            initial_vault_path.clone(),
            cache_path,
            file_types.clone(),
        );
        Self {
            active_vault_path: Mutex::new(initial_vault_path),
            file_types,
            note_use_cases,
            navigation_engine: Mutex::new(engine),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectVaultFolderResult {
    pub folder_path: String,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntryNode {
    pub name: String,
    pub relative_path: String,
    pub is_folder: bool,
    pub title: Option<String>,
}

#[tauri::command]
pub fn get_supported_file_types(state: State<'_, AppState>) -> SupportedFileTypes {
    state.file_types.clone()
}

#[tauri::command]
pub fn get_vault_notes(state: State<'_, AppState>) -> Result<Vec<Note>, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?.clone();
    let engine = state.navigation_engine.lock().map_err(|e| e.to_string())?.clone();

    let mut notes = Vec::with_capacity(engine.notes.len());
    for item in engine.notes.iter() {
        let meta = item.value();
        let rel_str = meta.path.to_string();
        let abs_path = vault_path.join(&rel_str).to_string_lossy().to_string();
        if let Ok(rel_path) = NoteRelativePath::new(&rel_str) {
            notes.push(Note::new(
                rel_path,
                abs_path,
                meta.title.to_string(),
                String::new(), // Zero content kept in memory for list metadata
            ));
        }
    }
    notes.sort_by(|a, b| a.relative_path.as_str().cmp(b.relative_path.as_str()));
    Ok(notes)
}

#[tauri::command]
pub fn get_vault_directory_children(
    state: State<'_, AppState>,
    parent_path: Option<String>,
) -> Result<Vec<VaultEntryNode>, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?.clone();
    let engine = state.navigation_engine.lock().map_err(|e| e.to_string())?.clone();

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
            if file_name.starts_with('.') {
                continue; // Skip hidden dirs like .git, .synapse
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
pub fn get_active_vault_path(state: State<'_, AppState>) -> Result<String, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    Ok(vault_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn read_note_content(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<Note, String> {
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let note = state.note_use_cases.read_note(&vault_path, &relative_path)?;
    if let Ok(engine) = state.navigation_engine.lock() {
        engine.record_opened(&relative_path);
    }
    Ok(note)
}

#[tauri::command]
pub fn record_note_opened(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    if let Ok(engine) = state.navigation_engine.lock() {
        engine.record_opened(&relative_path);
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
    let vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?.clone();
    let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());
    state
        .note_use_cases
        .save_note(&vault_path, &relative_path, &title, &content, &enc)?;

    if let Ok(engine) = state.navigation_engine.lock() {
        engine.reconcile_sync();
    }
    Ok(())
}

#[tauri::command]
pub fn set_active_vault_path(state: State<'_, AppState>, new_path: String) -> Result<(), String> {
    let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let path = PathBuf::from(&new_path);
    let target_dir = if path.is_dir() {
        path
    } else if path.is_file() {
        path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
    } else {
        path
    };

    *vault_path = target_dir.clone();
    let cache_path = target_dir.join(".synapse").join("cache.bin");
    let new_engine = NavigationEngine::start_with_file_types(
        target_dir,
        cache_path,
        state.file_types.clone(),
    );
    if let Ok(mut engine_lock) = state.navigation_engine.lock() {
        *engine_lock = new_engine;
    }
    Ok(())
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

        let cache_path = path.join(".synapse").join("cache.bin");
        let new_engine = NavigationEngine::start_with_file_types(
            path.clone(),
            cache_path,
            state.file_types.clone(),
        );
        if let Ok(mut engine_lock) = state.navigation_engine.lock() {
            *engine_lock = new_engine.clone();
        }

        let mut notes = Vec::with_capacity(new_engine.notes.len());
        for item in new_engine.notes.iter() {
            let meta = item.value();
            let rel_str = meta.path.to_string();
            let abs_path = path.join(&rel_str).to_string_lossy().to_string();
            if let Ok(rel_path) = NoteRelativePath::new(&rel_str) {
                notes.push(Note::new(
                    rel_path,
                    abs_path,
                    meta.title.to_string(),
                    String::new(),
                ));
            }
        }
        notes.sort_by(|a, b| a.relative_path.as_str().cmp(b.relative_path.as_str()));

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
pub fn search_notes_command(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SearchResult>, String> {
    let engine = state.navigation_engine.lock().map_err(|e| e.to_string())?.clone();
    let matches = engine.search(&query, 50);
    let recent_notes = engine.get_recent_notes(15);
    let recent_set: std::collections::HashSet<compact_str::CompactString> =
        recent_notes.into_iter().map(|n| n.path).collect();

    let results = matches
        .into_iter()
        .enumerate()
        .map(|(idx, m)| {
            let is_recent = recent_set.contains(&m.path);
            SearchResult {
                text: m.title.to_string(),
                score: (1000_u32).saturating_sub(idx as u32 * 10),
                match_indices: Vec::new(),
                note_path: Some(m.path.to_string()),
                is_recent: Some(is_recent),
                last_opened_nanos: m.last_opened_nanos,
            }
        })
        .collect();
    Ok(results)
}

#[tauri::command]
pub fn get_recent_notes_command(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let engine = state.navigation_engine.lock().map_err(|e| e.to_string())?.clone();
    let recent = engine.get_recent_notes(limit.unwrap_or(15));
    Ok(recent.into_iter().map(|n| n.path.to_string()).collect())
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
        None => state
            .active_vault_path
            .lock()
            .map_err(|e| e.to_string())?
            .clone(),
    };
    let git_service = GitService::new();
    git_service.get_vault_status(&vault_path)
}

#[tauri::command]
pub fn delete_vault_item(state: State<'_, AppState>, relative_path: String) -> Result<(), String> {
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
        std::fs::remove_dir_all(&canonical_target)
            .map_err(|e| format!("Error al eliminar carpeta: {}", e))?;
    } else if canonical_target.is_file() {
        std::fs::remove_file(&canonical_target)
            .map_err(|e| format!("Error al eliminar archivo: {}", e))?;
    } else {
        return Err("Tipo de elemento no soportado para eliminar".to_string());
    }

    if let Ok(engine) = state.navigation_engine.lock() {
        engine.reconcile_sync();
    }

    Ok(())
}

#[tauri::command]
pub fn git_add_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let vault_path = state
        .active_vault_path
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let git_service = GitService::new();
    git_service.git_add(&vault_path, &paths)
}

#[tauri::command]
pub fn git_restore_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let vault_path = state
        .active_vault_path
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let git_service = GitService::new();
    git_service.git_restore(&vault_path, &paths)
}

#[tauri::command]
pub fn git_restore_staged_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<(), String> {
    let vault_path = state
        .active_vault_path
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let git_service = GitService::new();
    git_service.git_restore_staged(&vault_path, &paths)
}

#[tauri::command]
pub fn get_system_theme(window: tauri::WebviewWindow) -> String {
    #[cfg(target_os = "linux")]
    {
        // 1. Try freedesktop portal via busctl (standard Wayland / XDG portal)
        if let Ok(output) = std::process::Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.freedesktop.portal.Desktop",
                "/org/freedesktop/portal/desktop",
                "org.freedesktop.portal.Settings",
                "Read",
                "ss",
                "org.freedesktop.appearance",
                "color-scheme",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("u 1") {
                    return "dark".to_string();
                } else if stdout.contains("u 2") {
                    return "light".to_string();
                }
            }
        }

        // 2. Try gsettings color-scheme (GNOME / Hyde / desktop interface)
        if let Ok(output) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_lowercase();
                if stdout.contains("dark") {
                    return "dark".to_string();
                } else if stdout.contains("light") {
                    return "light".to_string();
                }
            }
        }

        // 3. Try gsettings gtk-theme
        if let Ok(output) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
            .output()
        {
            if output.status.success() {
                let theme = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_lowercase();
                if theme.contains("dark") || theme.contains("black") || theme.contains("night") {
                    return "dark".to_string();
                } else if theme.contains("light") || theme.contains("white") {
                    return "light".to_string();
                }
            }
        }
    }

    if let Ok(theme) = window.theme() {
        return match theme {
            tauri::Theme::Dark => "dark".to_string(),
            tauri::Theme::Light => "light".to_string(),
            _ => "dark".to_string(),
        };
    }

    "dark".to_string()
}
