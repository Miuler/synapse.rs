use crate::application::use_cases::community_plugin_use_cases::CommunityPluginUseCases;
use crate::application::use_cases::file_system_use_cases::FileSystemUseCases;
use crate::application::use_cases::full_text_search_use_cases::FullTextSearchUseCases;
use crate::application::use_cases::git_use_cases::GitUseCases;
use crate::application::use_cases::note_use_cases::NoteUseCases;
use crate::application::use_cases::workspace_registry_use_cases::WorkspaceRegistryUseCases;
use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::note::Note;
use crate::domain::models::startup::StartupTarget;
use crate::domain::value_objects::note_path::NoteRelativePath;
use crate::infrastructure::navigation::engine::NavigationEngine;
use crate::infrastructure::repositories::file_note_repository::FileNoteRepository;
use crate::infrastructure::repositories::file_workspace_registry_repository::FileWorkspaceRegistryRepository;
use crate::infrastructure::search::fts_indexer::FtsIndexer;
use crate::infrastructure::search::tantivy_index::TantivyFullTextIndex;
use crate::infrastructure::mcp::server::{McpBackgroundRuntime, McpContextProvider};
use crate::domain::services::full_text_index::FullTextIndex;
use crate::infrastructure::services::file_system_service::FileSystemService;
use crate::infrastructure::services::git_service::GitService;
use crate::infrastructure::services::http_community_plugin_downloader::HttpCommunityPluginDownloader;
use crate::infrastructure::tauri::dto::SelectVaultFolderResult;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct FullTextComponents {
    pub use_cases: FullTextSearchUseCases<TantivyFullTextIndex>,
    pub indexer: Arc<FtsIndexer>,
}

pub struct DynamicMcpContextProvider {
    pub active_vault_path: Arc<Mutex<Option<PathBuf>>>,
    pub full_text: Arc<Mutex<Option<Arc<FullTextComponents>>>>,
}

impl McpContextProvider for DynamicMcpContextProvider {
    fn get_context(&self) -> (Option<PathBuf>, Option<Arc<dyn FullTextIndex>>) {
        let vault = self.active_vault_path.lock().ok().and_then(|g| g.clone());
        let fts: Option<Arc<dyn FullTextIndex>> = self.full_text.lock().ok().and_then(|g| {
            g.as_ref().map(|ft| ft.use_cases.index_arc() as Arc<dyn FullTextIndex>)
        });
        (vault, fts)
    }
}

pub struct AppState {
    pub active_vault_path: Arc<Mutex<Option<PathBuf>>>,
    pub file_types: SupportedFileTypes,
    pub note_use_cases: NoteUseCases<FileNoteRepository>,
    pub git_use_cases: GitUseCases<GitService>,
    pub file_system_use_cases: FileSystemUseCases<FileSystemService>,
    pub workspace_registry_use_cases: WorkspaceRegistryUseCases<FileWorkspaceRegistryRepository>,
    pub community_plugin_use_cases: CommunityPluginUseCases<HttpCommunityPluginDownloader>,
    pub navigation_engine: Mutex<Option<Arc<NavigationEngine>>>,
    pub full_text: Arc<Mutex<Option<Arc<FullTextComponents>>>>,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    pub detached_tabs: Arc<Mutex<HashMap<String, String>>>,
    pub startup_target: Arc<Mutex<Option<StartupTarget>>>,
    pub mcp_runtime: Arc<McpBackgroundRuntime>,
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

        let active_vault_path = Arc::new(Mutex::new(None));
        let full_text = Arc::new(Mutex::new(None));
        let app_handle: Arc<Mutex<Option<tauri::AppHandle>>> = Arc::new(Mutex::new(None));

        let mcp_provider = Arc::new(DynamicMcpContextProvider {
            active_vault_path: Arc::clone(&active_vault_path),
            full_text: Arc::clone(&full_text),
        });
        let mcp_runtime = Arc::new(McpBackgroundRuntime::new(mcp_provider));

        let handle_for_mcp = Arc::clone(&app_handle);
        mcp_runtime.set_on_status_change(Arc::new(move |status| {
            if let Ok(guard) = handle_for_mcp.lock() {
                if let Some(ref handle) = *guard {
                    use tauri::Emitter;
                    let _ = handle.emit("mcp:status-changed", &status);
                }
            }
        }));

        Self {
            active_vault_path,
            file_types,
            note_use_cases,
            git_use_cases: GitUseCases::new(GitService::new()),
            file_system_use_cases: FileSystemUseCases::new(FileSystemService),
            workspace_registry_use_cases,
            community_plugin_use_cases: CommunityPluginUseCases::new(HttpCommunityPluginDownloader::new()),
            navigation_engine: Mutex::new(None),
            full_text,
            app_handle,
            detached_tabs: Arc::new(Mutex::new(HashMap::new())),
            startup_target: Arc::new(Mutex::new(None)),
            mcp_runtime,
        }
    }

    pub fn new(
        initial_vault_path: PathBuf,
        file_types: SupportedFileTypes,
        note_use_cases: NoteUseCases<FileNoteRepository>,
    ) -> Self {
        let app_handle: Arc<Mutex<Option<tauri::AppHandle>>> = Arc::new(Mutex::new(None));
        let registry_repo = FileWorkspaceRegistryRepository::new(file_types.clone());
        let workspace_registry_use_cases = WorkspaceRegistryUseCases::new(registry_repo);
        let (engine, full_text_comps) = open_vault_components(
            initial_vault_path.clone(),
            file_types.clone(),
            Arc::clone(&app_handle),
        );

        let active_vault_path = Arc::new(Mutex::new(Some(initial_vault_path)));
        let full_text = Arc::new(Mutex::new(Some(full_text_comps)));

        let mcp_provider = Arc::new(DynamicMcpContextProvider {
            active_vault_path: Arc::clone(&active_vault_path),
            full_text: Arc::clone(&full_text),
        });
        let mcp_runtime = Arc::new(McpBackgroundRuntime::new(mcp_provider));

        let handle_for_mcp = Arc::clone(&app_handle);
        mcp_runtime.set_on_status_change(Arc::new(move |status| {
            if let Ok(guard) = handle_for_mcp.lock() {
                if let Some(ref handle) = *guard {
                    use tauri::Emitter;
                    let _ = handle.emit("mcp:status-changed", &status);
                }
            }
        }));

        Self {
            active_vault_path,
            file_types,
            note_use_cases,
            git_use_cases: GitUseCases::new(GitService::new()),
            file_system_use_cases: FileSystemUseCases::new(FileSystemService),
            workspace_registry_use_cases,
            community_plugin_use_cases: CommunityPluginUseCases::new(HttpCommunityPluginDownloader::new()),
            navigation_engine: Mutex::new(Some(engine)),
            full_text,
            app_handle,
            detached_tabs: Arc::new(Mutex::new(HashMap::new())),
            startup_target: Arc::new(Mutex::new(None)),
            mcp_runtime,
        }
    }

    pub fn activate_vault(&self, path: PathBuf, is_git: Option<bool>) -> Result<SelectVaultFolderResult, String> {
        let folder_path_str = path.to_string_lossy().to_string();

        let mut vault_path = self.active_vault_path.lock().map_err(|e| e.to_string())?;
        *vault_path = Some(path.clone());

        let (new_engine, new_ft) = open_vault_components(
            path.clone(),
            self.file_types.clone(),
            Arc::clone(&self.app_handle),
        );
        if let Ok(mut engine_lock) = self.navigation_engine.lock() {
            *engine_lock = Some(new_engine.clone());
        }
        if let Ok(mut ft_lock) = self.full_text.lock() {
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

        // Si el runtime de MCP está activo, notificar el cambio de bóveda al frontend
        if self.mcp_runtime.get_status().running {
            if let Ok(guard) = self.app_handle.lock() {
                if let Some(ref handle) = *guard {
                    use tauri::Emitter;
                    let _ = handle.emit("mcp:status-changed", &self.mcp_runtime.get_status());
                }
            }
        }

        let _ = self.workspace_registry_use_cases.record_workspace_opened(
            &folder_path_str,
            Some(notes.len()),
            None,
            is_git,
        );

        Ok(SelectVaultFolderResult {
            folder_path: folder_path_str,
            notes,
        })
    }
}
