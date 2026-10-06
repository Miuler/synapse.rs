use crate::application::use_cases::full_text_search_use_cases::FullTextSearchUseCases;
use crate::application::use_cases::note_use_cases::NoteUseCases;
use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::full_text::{FullTextIndexStatus, FullTextSearchResponse};
use crate::domain::models::note::Note;
use crate::domain::services::search_service::{SearchResult, SearchService};
use crate::domain::value_objects::note_path::NoteRelativePath;
use crate::infrastructure::repositories::file_note_repository::FileNoteRepository;
use crate::infrastructure::search::fts_indexer::FtsIndexer;
use crate::infrastructure::search::tantivy_index::TantivyFullTextIndex;
use crate::infrastructure::services::file_system_service::FileSystemService;
use crate::infrastructure::services::git_service::{GitService, VaultGitStatus};
use crate::infrastructure::services::nucleo_search_service::NucleoSearchService;
use crate::navigation::engine::{NavigationEngine, OpenTabDto, VaultUiState, WorkspaceOpenTabsState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct FullTextComponents {
    pub use_cases: FullTextSearchUseCases<TantivyFullTextIndex>,
    pub indexer: Arc<FtsIndexer>,
}

pub struct AppState {
    pub active_vault_path: Mutex<Option<PathBuf>>,
    pub file_types: SupportedFileTypes,
    pub note_use_cases: NoteUseCases<FileNoteRepository>,
    pub navigation_engine: Mutex<Option<Arc<NavigationEngine>>>,
    pub full_text: Mutex<Option<Arc<FullTextComponents>>>,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

pub fn open_vault_components(
    vault_path: PathBuf,
    file_types: SupportedFileTypes,
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
) -> (Arc<NavigationEngine>, Arc<FullTextComponents>) {
    let cache_path = vault_path.join(".synapse").join("cache.bin");
    let engine = NavigationEngine::start_with_file_types(
        vault_path.clone(),
        cache_path,
        file_types.clone(),
    );

    let handle_for_fs = Arc::clone(&app_handle);
    engine.set_on_fs_change(Arc::new(move |evt| {
        if let Ok(guard) = handle_for_fs.lock() {
            if let Some(ref handle) = *guard {
                use tauri::Emitter;
                let _ = handle.emit("vault:files-changed", &evt);
            }
        }
    }));

    let fts_dir = vault_path.join(".synapse").join("fts");
    let tantivy_index = match TantivyFullTextIndex::open_or_create(&fts_dir) {
        Ok(idx) => Arc::new(idx),
        Err(e) => {
            log::warn!("No se pudo abrir índice FTS en disco ({}), usando fallback en RAM", e);
            Arc::new(TantivyFullTextIndex::in_memory().expect("Fallo al crear índice FTS en RAM"))
        }
    };

    let indexer = FtsIndexer::start(vault_path, file_types, Arc::clone(&tantivy_index));
    let handle_for_fts = Arc::clone(&app_handle);
    indexer.set_on_status_change(Arc::new(move |status| {
        if let Ok(guard) = handle_for_fts.lock() {
            if let Some(ref handle) = *guard {
                use tauri::Emitter;
                let _ = handle.emit("vault:indexing-status", &status);
            }
        }
    }));
    engine.register_observer(Arc::clone(&indexer) as Arc<dyn crate::domain::events::vault_events::VaultChangeObserver>);

    let use_cases = FullTextSearchUseCases::new(tantivy_index);
    let full_text = Arc::new(FullTextComponents {
        use_cases,
        indexer,
    });

    (engine, full_text)
}

impl AppState {
    pub fn empty(
        file_types: SupportedFileTypes,
        note_use_cases: NoteUseCases<FileNoteRepository>,
    ) -> Self {
        Self {
            active_vault_path: Mutex::new(None),
            file_types,
            note_use_cases,
            navigation_engine: Mutex::new(None),
            full_text: Mutex::new(None),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new(
        initial_vault_path: PathBuf,
        file_types: SupportedFileTypes,
        note_use_cases: NoteUseCases<FileNoteRepository>,
    ) -> Self {
        let app_handle = Arc::new(Mutex::new(None));
        let (engine, full_text) = open_vault_components(
            initial_vault_path.clone(),
            file_types.clone(),
            Arc::clone(&app_handle),
        );
        Self {
            active_vault_path: Mutex::new(Some(initial_vault_path)),
            file_types,
            note_use_cases,
            navigation_engine: Mutex::new(Some(engine)),
            full_text: Mutex::new(Some(full_text)),
            app_handle,
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
            if crate::navigation::watcher::is_ignored_dir_or_file(&file_name) {
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
    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *vault_guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let note = state.note_use_cases.read_note(vault_path, &relative_path)?;
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.record_opened(&relative_path);
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
    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *vault_guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let enc = encoding.unwrap_or_else(|| "UTF-8".to_string());
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

    let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    *vault_path = Some(target_dir.clone());
    let (new_engine, new_ft) = open_vault_components(
        target_dir,
        state.file_types.clone(),
        Arc::clone(&state.app_handle),
    );
    if let Ok(mut engine_lock) = state.navigation_engine.lock() {
        *engine_lock = Some(new_engine);
    }
    if let Ok(mut ft_lock) = state.full_text.lock() {
        *ft_lock = Some(new_ft);
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
        let folder_path_str = path.to_string_lossy().to_string();

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

        let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        *vault_path = Some(path.clone());

        let (new_engine, new_ft) = open_vault_components(
            path.clone(),
            state.file_types.clone(),
            Arc::clone(&state.app_handle),
        );
        if let Ok(mut engine_lock) = state.navigation_engine.lock() {
            *engine_lock = Some(new_engine.clone());
        }
        if let Ok(mut ft_lock) = state.full_text.lock() {
            *ft_lock = Some(new_ft);
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuickOpenSearchResult {
    pub results: Vec<SearchResult>,
    pub total_files: usize,
    pub matched_files: usize,
}

#[tauri::command]
pub fn search_notes_command(
    state: State<'_, AppState>,
    query: String,
) -> Result<QuickOpenSearchResult, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(QuickOpenSearchResult {
            results: Vec::new(),
            total_files: 0,
            matched_files: 0,
        });
    };
    let (matches, total_files, matched_files) = engine.search_with_stats(&query, 50);
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
    Ok(QuickOpenSearchResult {
        results,
        total_files,
        matched_files,
    })
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
) -> Result<(), String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *guard {
        engine.save_vault_ui_state(sidebar_width, expanded_folders);
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
    let git_service = GitService::new();
    git_service.get_vault_status(&vault_path)
}

#[tauri::command]
pub fn delete_vault_item(state: State<'_, AppState>, relative_path: String) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
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

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }

    Ok(())
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

    let mut created = Vec::with_capacity(paths.len());
    for source in &paths {
        created.push(FileSystemService::copy_item(&vault_path, source, &dest_dir)?);
    }

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            // Indexa sólo lo creado (incluye contenido de carpetas) y notifica al frontend
            let _ = engine.reload_paths(&created);
        }
    }

    Ok(created)
}

#[tauri::command]
pub fn git_add_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let git_service = GitService::new();
    git_service.git_add(vault_path, &paths)
}

#[tauri::command]
pub fn git_restore_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let git_service = GitService::new();
    git_service.git_restore(vault_path, &paths)
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
    let git_service = GitService::new();
    git_service.git_restore_staged(vault_path, &paths)
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

#[tauri::command]
pub async fn full_text_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<FullTextSearchResponse, String> {
    let ft_components = {
        let guard = state.full_text.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };
    let Some(ft_components) = ft_components else {
        return Ok(FullTextSearchResponse {
            hits: Vec::new(),
            total_hits: 0,
            elapsed_ms: 0.0,
            status: FullTextIndexStatus::default(),
        });
    };

    let start = std::time::Instant::now();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let (hits, total_hits) = ft_components.use_cases.search(&query, limit)?;
        let status = ft_components.indexer.status();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        Ok(FullTextSearchResponse {
            hits,
            total_hits,
            elapsed_ms,
            status,
        })
    })
    .await
    .map_err(|e| format!("Error en tarea de búsqueda: {}", e))?;

    res
}

#[tauri::command]
pub fn get_full_text_index_status(state: State<'_, AppState>) -> Result<FullTextIndexStatus, String> {
    let guard = state.full_text.lock().map_err(|e| e.to_string())?;
    let Some(ref ft_components) = *guard else {
        return Ok(FullTextIndexStatus::default());
    };
    Ok(ft_components.indexer.status())
}

#[tauri::command]
pub fn rebuild_full_text_index(state: State<'_, AppState>) -> Result<(), String> {
    let guard = state.full_text.lock().map_err(|e| e.to_string())?;
    let Some(ref ft_components) = *guard else {
        return Ok(());
    };
    ft_components.indexer.rebuild()
}


