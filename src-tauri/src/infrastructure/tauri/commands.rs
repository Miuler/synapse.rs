use crate::application::use_cases::community_plugin_use_cases::CommunityPluginUseCases;
use crate::application::use_cases::file_system_use_cases::FileSystemUseCases;
use crate::application::use_cases::full_text_search_use_cases::FullTextSearchUseCases;
use crate::application::use_cases::git_use_cases::GitUseCases;
use crate::application::use_cases::note_use_cases::NoteUseCases;
use crate::application::use_cases::workspace_registry_use_cases::WorkspaceRegistryUseCases;
use crate::domain::models::community_plugin::{
    CommunityPlugin, CommunityPluginInstallResult, InstalledPluginInfo, PluginUpdateInfo,
    VaultDirectoryList,
};
use crate::domain::models::config::ShortcutsConfig;
use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::full_text::{FullTextIndexStatus, FullTextSearchResponse};
use crate::domain::models::note::Note;
use crate::domain::models::startup::StartupTarget;
use crate::domain::models::workspace_registry::RecentWorkspaceEntry;
use crate::domain::services::search_service::{SearchResult, SearchService};
use crate::domain::value_objects::note_path::NoteRelativePath;
use crate::infrastructure::repositories::file_note_repository::FileNoteRepository;
use crate::infrastructure::repositories::file_workspace_registry_repository::FileWorkspaceRegistryRepository;
use crate::infrastructure::search::fts_indexer::FtsIndexer;
use crate::infrastructure::search::tantivy_index::TantivyFullTextIndex;
use crate::infrastructure::services::file_system_service::FileSystemService;
use crate::infrastructure::services::http_community_plugin_downloader::HttpCommunityPluginDownloader;
use crate::infrastructure::services::git_service::{
    GitBranchesResult, GitDiffResponse, GitFileLogResult, GitFileRevisionResult, GitImageDiffResult,
    GitService, VaultGitStatus,
};
use crate::infrastructure::services::nucleo_search_service::NucleoSearchService;
use crate::infrastructure::navigation::engine::{NavigationEngine, OpenTabDto, VaultUiState, WorkspaceOpenTabsState};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
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
    pub git_use_cases: GitUseCases<GitService>,
    pub file_system_use_cases: FileSystemUseCases<FileSystemService>,
    pub workspace_registry_use_cases: WorkspaceRegistryUseCases<FileWorkspaceRegistryRepository>,
    pub community_plugin_use_cases: CommunityPluginUseCases<HttpCommunityPluginDownloader>,
    pub navigation_engine: Mutex<Option<Arc<NavigationEngine>>>,
    pub full_text: Mutex<Option<Arc<FullTextComponents>>>,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    pub detached_tabs: Arc<Mutex<HashMap<String, String>>>,
    pub startup_target: Arc<Mutex<Option<StartupTarget>>>,
}

pub use crate::infrastructure::navigation::model::{VaultFileDto, VaultFileStatDto};
use crate::infrastructure::navigation::engine::VaultFilesStream;
use serde::ser::{SerializeSeq, Serializer};

pub struct VaultFilesResponse {
    pub engine: Option<Arc<NavigationEngine>>,
}

impl Serialize for VaultFilesResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(engine) = &self.engine {
            VaultFilesStream {
                engine: Arc::clone(engine),
            }
            .serialize(serializer)
        } else {
            let seq = serializer.serialize_seq(Some(0))?;
            seq.end()
        }
    }
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
    if let Ok(guard) = app_handle.lock() {
        if let Some(ref handle) = *guard {
            use tauri::Manager;
            let _ = handle.asset_protocol_scope().allow_directory(&vault_path, true);
        }
    }
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
        let registry_repo = FileWorkspaceRegistryRepository::new(file_types.clone());
        let workspace_registry_use_cases = WorkspaceRegistryUseCases::new(registry_repo);
        // Ensure ~/.config/synapsers/workspaces.json and shortcuts.jsonc exist on startup
        let _ = workspace_registry_use_cases.get_registry();
        let _ = workspace_registry_use_cases.get_shortcuts_config();
        Self {
            active_vault_path: Mutex::new(None),
            file_types,
            note_use_cases,
            git_use_cases: GitUseCases::new(GitService::new()),
            file_system_use_cases: FileSystemUseCases::new(FileSystemService),
            workspace_registry_use_cases,
            community_plugin_use_cases: CommunityPluginUseCases::new(HttpCommunityPluginDownloader::new()),
            navigation_engine: Mutex::new(None),
            full_text: Mutex::new(None),
            app_handle: Arc::new(Mutex::new(None)),
            detached_tabs: Arc::new(Mutex::new(HashMap::new())),
            startup_target: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new(
        initial_vault_path: PathBuf,
        file_types: SupportedFileTypes,
        note_use_cases: NoteUseCases<FileNoteRepository>,
    ) -> Self {
        let app_handle = Arc::new(Mutex::new(None));
        let registry_repo = FileWorkspaceRegistryRepository::new(file_types.clone());
        let workspace_registry_use_cases = WorkspaceRegistryUseCases::new(registry_repo);
        let (engine, full_text) = open_vault_components(
            initial_vault_path.clone(),
            file_types.clone(),
            Arc::clone(&app_handle),
        );
        Self {
            active_vault_path: Mutex::new(Some(initial_vault_path)),
            file_types,
            note_use_cases,
            git_use_cases: GitUseCases::new(GitService::new()),
            file_system_use_cases: FileSystemUseCases::new(FileSystemService),
            workspace_registry_use_cases,
            community_plugin_use_cases: CommunityPluginUseCases::new(HttpCommunityPluginDownloader::new()),
            navigation_engine: Mutex::new(Some(engine)),
            full_text: Mutex::new(Some(full_text)),
            app_handle,
            detached_tabs: Arc::new(Mutex::new(HashMap::new())),
            startup_target: Arc::new(Mutex::new(None)),
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

    let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    *vault_path = Some(target_dir.clone());
    let (new_engine, new_ft) = open_vault_components(
        target_dir.clone(),
        state.file_types.clone(),
        Arc::clone(&state.app_handle),
    );
    let notes_count = new_engine.notes.len();
    if let Ok(mut engine_lock) = state.navigation_engine.lock() {
        *engine_lock = Some(new_engine);
    }
    if let Ok(mut ft_lock) = state.full_text.lock() {
        *ft_lock = Some(new_ft);
    }
    let _ = state.workspace_registry_use_cases.record_workspace_opened(
        &target_dir.to_string_lossy(),
        Some(notes_count),
        None,
        None,
    );
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

        let _ = state.workspace_registry_use_cases.record_workspace_opened(
            &folder_path_str,
            Some(notes.len()),
            None,
            None,
        );

        Ok(Some(SelectVaultFolderResult {
            folder_path: folder_path_str,
            notes,
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn get_startup_target(state: State<'_, AppState>) -> Result<Option<StartupTarget>, String> {
    let mut guard = state.startup_target.lock().map_err(|e| e.to_string())?;
    Ok(guard.take())
}

#[tauri::command]
pub async fn select_standalone_file(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let file = rfd::AsyncFileDialog::new()
        .set_title("Abrir archivo")
        .add_filter(
            "Archivos soportados (*.md, *.mermaid, *.excalidraw)",
            &["md", "markdown", "mermaid", "mmd", "excalidraw", "json"],
        )
        .add_filter("Markdown (*.md, *.markdown)", &["md", "markdown"])
        .add_filter("Diagramas Mermaid (*.mermaid, *.mmd)", &["mermaid", "mmd"])
        .add_filter(
            "Dibujos Excalidraw (*.excalidraw, *.excalidraw.md, *.excalidraw.json)",
            &["excalidraw", "json", "md"],
        )
        .add_filter("Todos los archivos (*.*)", &["*"])
        .pick_file()
        .await;

    if let Some(handle) = file {
        let path = handle.path().to_string_lossy().to_string();
        if let Ok(guard) = state.app_handle.lock() {
            if let Some(ref app_h) = *guard {
                if let Some(parent) = handle.path().parent() {
                    use tauri::Manager;
                    let _ = app_h.asset_protocol_scope().allow_directory(parent, true);
                }
            }
        }
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

pub fn parse_startup_args(args: &[String]) -> Option<StartupTarget> {
    if args.len() <= 1 {
        return None;
    }

    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--file" || arg == "-f" {
            if let Some(target) = iter.next() {
                let p = Path::new(target);
                let resolved = if p.is_relative() {
                    std::env::current_dir().map(|cwd| cwd.join(p)).unwrap_or_else(|_| p.to_path_buf())
                } else {
                    p.to_path_buf()
                };
                let canonical = resolved.canonicalize().unwrap_or(resolved);
                return Some(StartupTarget::Standalone(canonical.to_string_lossy().to_string()));
            }
        } else if arg == "--vault" || arg == "-v" {
            if let Some(target) = iter.next() {
                let p = Path::new(target);
                let resolved = if p.is_relative() {
                    std::env::current_dir().map(|cwd| cwd.join(p)).unwrap_or_else(|_| p.to_path_buf())
                } else {
                    p.to_path_buf()
                };
                let canonical = resolved.canonicalize().unwrap_or(resolved);
                return Some(StartupTarget::Vault(canonical.to_string_lossy().to_string()));
            }
        } else if !arg.starts_with('-') {
            let p = Path::new(arg);
            let resolved = if p.is_relative() {
                std::env::current_dir().map(|cwd| cwd.join(p)).unwrap_or_else(|_| p.to_path_buf())
            } else {
                p.to_path_buf()
            };
            let canonical = resolved.canonicalize().unwrap_or(resolved);
            if canonical.is_dir() {
                return Some(StartupTarget::Vault(canonical.to_string_lossy().to_string()));
            } else {
                return Some(StartupTarget::Standalone(canonical.to_string_lossy().to_string()));
            }
        }
    }
    None
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
    state.git_use_cases.get_status(&vault_path)
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
pub fn open_file_with_default_app(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state.file_system_use_cases.open_file_with_default_app(vault_path, &path)
}

#[tauri::command]
pub fn git_add_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state.git_use_cases.add_paths(vault_path, &paths).map_err(|err| {
        log::error!("[commands::git_add_paths] {}", err);
        eprintln!("[commands::git_add_paths] {}", err);
        err
    })
}

#[tauri::command]
pub fn git_restore_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state.git_use_cases.restore_paths(vault_path, &paths).map_err(|err| {
        log::error!("[commands::git_restore_paths] {}", err);
        eprintln!("[commands::git_restore_paths] {}", err);
        err
    })
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
    state.git_use_cases.restore_staged_paths(vault_path, &paths).map_err(|err| {
        log::error!("[commands::git_restore_staged_paths] {}", err);
        eprintln!("[commands::git_restore_staged_paths] {}", err);
        err
    })
}

#[tauri::command]
pub fn git_commit_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
    message: String,
) -> Result<String, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state.git_use_cases.commit_paths(vault_path, &paths, &message).map_err(|err| {
        log::error!("[commands::git_commit_paths] {}", err);
        eprintln!("[commands::git_commit_paths] {}", err);
        err
    })
}

#[tauri::command]
pub fn get_git_file_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<GitDiffResponse, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .git_use_cases
        .get_file_diff(vault_path, &path, staged, revision.as_deref())
        .map_err(|err| {
            log::error!("[commands::get_git_file_diff] {}", err);
            eprintln!("[commands::get_git_file_diff] {}", err);
            err
        })
}

#[tauri::command]
pub fn get_git_file_revision(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<GitFileRevisionResult, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let default_rev = if staged { "staged" } else { "HEAD" };
    let rev = revision.as_deref().unwrap_or(default_rev);
    state
        .git_use_cases
        .get_file_revision(vault_path, &path, rev)
        .map_err(|err| {
            log::error!("[commands::get_git_file_revision] {}", err);
            eprintln!("[commands::get_git_file_revision] {}", err);
            err
        })
}

#[tauri::command]
pub fn get_git_image_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<GitImageDiffResult, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .git_use_cases
        .get_image_diff(vault_path, &path, staged, revision.as_deref())
        .map_err(|err| {
            log::error!("[commands::get_git_image_diff] {}", err);
            eprintln!("[commands::get_git_image_diff] {}", err);
            err
        })
}

#[tauri::command]
pub fn get_git_file_log(
    state: State<'_, AppState>,
    path: String,
    max_count: Option<usize>,
) -> Result<GitFileLogResult, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .git_use_cases
        .get_file_log(vault_path, &path, max_count)
        .map_err(|err| {
            log::error!("[commands::get_git_file_log] {}", err);
            eprintln!("[commands::get_git_file_log] {}", err);
            err
        })
}

#[tauri::command]
pub fn get_git_branches(state: State<'_, AppState>) -> Result<GitBranchesResult, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state.git_use_cases.get_branches(vault_path)
}

#[tauri::command]
pub fn git_checkout_branch(
    state: State<'_, AppState>,
    branch_name: String,
) -> Result<String, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let result = state.git_use_cases.checkout_branch(vault_path, &branch_name)?;

    // Sincronizar el árbol de navegación ya que los archivos cambiaron en disco
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn git_create_branch(
    state: State<'_, AppState>,
    new_branch: String,
    base_branch: String,
    checkout: bool,
) -> Result<String, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let result = state
        .git_use_cases
        .create_branch(vault_path, &new_branch, &base_branch, checkout)?;

    if checkout {
        if let Ok(guard) = state.navigation_engine.lock() {
            if let Some(ref engine) = *guard {
                engine.reconcile_sync();
            }
        }
    }

    Ok(result)
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

#[tauri::command]
pub fn resolve_vault_link(
    state: State<'_, AppState>,
    link: String,
    base_file: Option<String>,
) -> Result<Option<String>, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(None);
    };
    Ok(engine.resolve_link_path_with_base(&link, base_file.as_deref()))
}

#[tauri::command]
pub fn resolve_vault_links(
    state: State<'_, AppState>,
    links: Vec<String>,
    base_file: Option<String>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(std::collections::HashMap::new());
    };
    Ok(engine.resolve_link_paths_with_base(&links, base_file.as_deref()))
}

#[tauri::command]
pub fn render_markdown_wikilinks(
    state: State<'_, AppState>,
    content: String,
    base_file: Option<String>,
) -> Result<String, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(content);
    };
    Ok(engine.render_wikilinks_in_markdown_with_base(&content, base_file.as_deref()))
}

fn percent_decode_str(s: &str) -> String {
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

fn get_image_mime_type(ext: &str) -> &'static str {
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

#[tauri::command]
pub fn open_tab_in_new_window(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    title: Option<String>,
) -> Result<(), String> {
    let window_title = title.unwrap_or_else(|| {
        Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Synapse")
            .to_string()
    });

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let window_label = format!("win_{}", timestamp);

    // Registrar en detached_tabs para que la ventana sepa qué archivo cargar
    if let Ok(mut map) = state.detached_tabs.lock() {
        map.insert(window_label.clone(), path);
    }

    let win = tauri::WebviewWindowBuilder::new(
        &app,
        &window_label,
        tauri::WebviewUrl::default(),
    )
    .title(format!("Synapse - {}", window_title))
    .inner_size(900.0, 700.0)
    .decorations(false)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("Error creando nueva ventana: {}", e))?;

    let tabs_map = Arc::clone(&state.detached_tabs);
    let label_for_event = window_label.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            if let Ok(mut map) = tabs_map.lock() {
                map.remove(&label_for_event);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn get_detached_window_tab(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let guard = state.detached_tabs.lock().map_err(|e| e.to_string())?;
    Ok(guard.get(window.label()).cloned())
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
        let folder_path_str = path.to_string_lossy().to_string();

        let synapse_dir = path.join(".synapse");
        if let Err(e) = std::fs::create_dir_all(&synapse_dir) {
            return Err(format!("No se pudo crear el directorio .synapse: {}", e));
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

        let _ = state.workspace_registry_use_cases.record_workspace_opened(
            &folder_path_str,
            Some(notes.len()),
            None,
            None,
        );

        Ok(Some(SelectVaultFolderResult {
            folder_path: folder_path_str,
            notes,
        }))
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

    let output = std::process::Command::new("git")
        .arg("clone")
        .arg(&trimmed_url)
        .arg(&target_dir)
        .output()
        .map_err(|e| format!("Error al ejecutar git clone: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone falló: {}", stderr.trim()));
    }

    let synapse_dir = target_dir.join(".synapse");
    let _ = std::fs::create_dir_all(&synapse_dir);

    let folder_path_str = target_dir.to_string_lossy().to_string();

    let mut vault_path = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    *vault_path = Some(target_dir.clone());

    let (new_engine, new_ft) = open_vault_components(
        target_dir.clone(),
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
        let abs_path = target_dir.join(&rel_str).to_string_lossy().to_string();
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

    let _ = state.workspace_registry_use_cases.record_workspace_opened(
        &folder_path_str,
        Some(notes.len()),
        None,
        Some(true),
    );

    Ok(Some(SelectVaultFolderResult {
        folder_path: folder_path_str,
        notes,
    }))
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
pub async fn fetch_community_plugins(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<CommunityPlugin>, String> {
    state
        .community_plugin_use_cases
        .fetch_community_plugins(force_refresh.unwrap_or(false))
        .await
}

#[tauri::command]
pub fn get_installed_community_plugins(
    state: State<'_, AppState>,
    vault_path: Option<String>,
) -> Result<Vec<InstalledPluginInfo>, String> {
    let resolved_path = match vault_path {
        Some(p) => PathBuf::from(p),
        None => state
            .active_vault_path
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No hay ninguna bóveda activa seleccionada".to_string())?,
    };

    state
        .community_plugin_use_cases
        .get_installed_plugins(&resolved_path)
}

#[tauri::command]
pub async fn check_community_plugin_updates(
    state: State<'_, AppState>,
    plugins: Vec<(String, String, String)>,
) -> Result<Vec<PluginUpdateInfo>, String> {
    state
        .community_plugin_use_cases
        .check_plugin_updates(&plugins)
        .await
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

#[tauri::command]
pub async fn install_community_plugin(
    state: State<'_, AppState>,
    vault_path: Option<String>,
    plugin_id: String,
    repo: String,
) -> Result<CommunityPluginInstallResult, String> {
    let resolved_path = match vault_path {
        Some(p) => PathBuf::from(p),
        None => state
            .active_vault_path
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No hay ninguna bóveda activa seleccionada".to_string())?,
    };

    state
        .community_plugin_use_cases
        .install_plugin(&resolved_path, &plugin_id, &repo)
        .await
}

#[tauri::command]
pub async fn uninstall_community_plugin(
    state: State<'_, AppState>,
    vault_path: Option<String>,
    plugin_id: String,
) -> Result<(), String> {
    let resolved_path = match vault_path {
        Some(p) => PathBuf::from(p),
        None => state
            .active_vault_path
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No hay ninguna bóveda activa seleccionada".to_string())?,
    };

    state
        .community_plugin_use_cases
        .uninstall_plugin(&resolved_path, &plugin_id)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_parse_startup_args() {
        let temp_dir = std::env::temp_dir().join("synapse_startup_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let file_path = temp_dir.join("note.md");
        std::fs::write(&file_path, "# Hola").unwrap();

        // 1. Sin argumentos
        let args_empty = vec!["synapsers".to_string()];
        assert_eq!(parse_startup_args(&args_empty), None);

        // 2. Argumento posicional de archivo existente
        let args_file = vec!["synapsers".to_string(), file_path.to_str().unwrap().to_string()];
        let target_file = parse_startup_args(&args_file);
        assert!(matches!(target_file, Some(StartupTarget::Standalone(_))));

        // 3. Flag --file
        let args_flag_file = vec![
            "synapsers".to_string(),
            "--file".to_string(),
            file_path.to_str().unwrap().to_string(),
        ];
        let target_flag = parse_startup_args(&args_flag_file);
        assert!(matches!(target_flag, Some(StartupTarget::Standalone(_))));

        // 4. Argumento posicional de directorio existente
        let args_dir = vec!["synapsers".to_string(), temp_dir.to_str().unwrap().to_string()];
        let target_dir = parse_startup_args(&args_dir);
        assert!(matches!(target_dir, Some(StartupTarget::Vault(_))));

        // 5. Flag --vault
        let args_flag_vault = vec![
            "synapsers".to_string(),
            "-v".to_string(),
            temp_dir.to_str().unwrap().to_string(),
        ];
        let target_flag_vault = parse_startup_args(&args_flag_vault);
        assert!(matches!(target_flag_vault, Some(StartupTarget::Vault(_))));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

