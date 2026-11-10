use crate::application::dto::search::index_vault_change_command::IndexVaultChangeCommand;
use crate::application::dto::workspace::get_shortcuts_config_query::GetShortcutsConfigQuery;
use crate::application::dto::workspace::get_workspace_registry_query::GetWorkspaceRegistryQuery;
use crate::application::dto::workspace::record_workspace_opened_command::RecordWorkspaceOpenedCommand;
use crate::application::use_cases::note::list_notes_use_case::ListNotesUseCase;
use crate::application::use_cases::note::read_note_use_case::ReadNoteUseCase;
use crate::application::use_cases::note::read_standalone_note_use_case::ReadStandaloneNoteUseCase;
use crate::application::use_cases::note::save_note_use_case::SaveNoteUseCase;
use crate::application::use_cases::note::save_standalone_note_use_case::SaveStandaloneNoteUseCase;
use crate::application::use_cases::plugin::check_plugin_updates_use_case::CheckPluginUpdatesUseCase;
use crate::application::use_cases::plugin::fetch_community_plugins_use_case::FetchCommunityPluginsUseCase;
use crate::application::use_cases::plugin::get_installed_plugins_use_case::GetInstalledPluginsUseCase;
use crate::application::use_cases::plugin::install_plugin_use_case::InstallPluginUseCase;
use crate::application::use_cases::plugin::uninstall_plugin_use_case::UninstallPluginUseCase;
use crate::application::use_cases::search::index_vault_change_use_case::IndexVaultChangeUseCase;
use crate::application::use_cases::search::search_full_text_use_case::SearchFullTextUseCase;
use crate::application::use_cases::session::get_note_view_mode_use_case::GetNoteViewModeUseCase;
use crate::application::use_cases::session::get_open_tabs_use_case::GetOpenTabsUseCase;
use crate::application::use_cases::session::get_recent_notes_use_case::GetRecentNotesUseCase;
use crate::application::use_cases::session::get_vault_ui_state_use_case::GetVaultUiStateUseCase;
use crate::application::use_cases::session::record_note_opened_use_case::RecordNoteOpenedUseCase;
use crate::application::use_cases::session::save_open_tabs_use_case::SaveOpenTabsUseCase;
use crate::application::use_cases::session::save_vault_ui_state_use_case::SaveVaultUiStateUseCase;
use crate::application::use_cases::session::set_note_view_mode_use_case::SetNoteViewModeUseCase;
use crate::application::use_cases::system::open_file_with_default_app_use_case::OpenFileWithDefaultAppUseCase;
use crate::application::use_cases::system::show_item_in_folder_use_case::ShowItemInFolderUseCase;
use crate::application::use_cases::vault_item::copy_vault_items_use_case::CopyVaultItemsUseCase;
use crate::application::use_cases::vault_item::create_folder_use_case::CreateFolderUseCase;
use crate::application::use_cases::vault_item::delete_vault_item_use_case::DeleteVaultItemUseCase;
use crate::application::use_cases::vault_item::move_vault_items_use_case::MoveVaultItemsUseCase;
use crate::application::use_cases::vault_item::read_binary_vault_item_use_case::ReadBinaryVaultItemUseCase;
use crate::application::use_cases::vault_item::rename_vault_item_use_case::RenameVaultItemUseCase;
use crate::application::use_cases::vault_item::write_binary_vault_item_use_case::WriteBinaryVaultItemUseCase;
use crate::application::use_cases::version::checkout_branch_use_case::CheckoutBranchUseCase;
use crate::application::use_cases::version::clone_repository_use_case::CloneRepositoryUseCase;
use crate::application::use_cases::version::create_branch_use_case::CreateBranchUseCase;
use crate::application::use_cases::version::get_branches_use_case::GetBranchesUseCase;
use crate::application::use_cases::version::get_file_diff_use_case::GetFileDiffUseCase;
use crate::application::use_cases::version::get_file_revision_use_case::GetFileRevisionUseCase;
use crate::application::use_cases::version::get_file_version_history_use_case::GetFileVersionHistoryUseCase;
use crate::application::use_cases::version::get_image_diff_use_case::GetImageDiffUseCase;
use crate::application::use_cases::version::get_vault_version_state_use_case::GetVaultVersionStateUseCase;
use crate::application::use_cases::version::push_versions_use_case::PushVersionsUseCase;
use crate::application::use_cases::version::revert_files_use_case::RevertFilesUseCase;
use crate::application::use_cases::version::save_version_use_case::SaveVersionUseCase;
use crate::application::use_cases::version::track_files_use_case::TrackFilesUseCase;
use crate::application::use_cases::version::untrack_files_use_case::UntrackFilesUseCase;
use crate::application::use_cases::workspace::get_shortcuts_config_use_case::GetShortcutsConfigUseCase;
use crate::application::use_cases::workspace::save_shortcut_use_case::SaveShortcutUseCase;
use crate::application::use_cases::workspace::get_workspace_registry_use_case::GetWorkspaceRegistryUseCase;
use crate::application::use_cases::workspace::list_workspaces_use_case::ListWorkspacesUseCase;
use crate::application::use_cases::workspace::record_workspace_opened_use_case::RecordWorkspaceOpenedUseCase;
use crate::application::use_cases::workspace::remove_workspace_use_case::RemoveWorkspaceUseCase;
use crate::application::use_cases::workspace::toggle_workspace_pin_use_case::ToggleWorkspacePinUseCase;
use crate::domain::config::startup::StartupTarget;
use crate::domain::search::full_text_index::FullTextIndex;
use crate::domain::vault::file_types::SupportedFileTypes;
use crate::infrastructure::entrypoint::mcp::server::{McpBackgroundRuntime, McpContextProvider};
use crate::infrastructure::entrypoint::tauri::dto::{SelectVaultFolderResult, VaultEntryDto};
use crate::infrastructure::navigation::engine::NavigationEngine;
use crate::infrastructure::repository::file::installed_plugin_repository_impl::InstalledPluginRepositoryImpl;
use crate::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use crate::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use crate::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use crate::infrastructure::repository::navigation::workspace_session_repository_impl::WorkspaceSessionRepositoryImpl;
use crate::infrastructure::repository::tantivy::full_text_index_impl::FullTextIndexImpl;
use crate::infrastructure::search::fts_indexer::FtsIndexer;
use crate::infrastructure::service::http::community_plugin_downloader_impl::CommunityPluginDownloaderImpl;
use crate::infrastructure::service::os::system_shell_impl::SystemShellImpl;

use dashmap::DashMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct FullTextComponents {
    pub search_use_case: SearchFullTextUseCase<FullTextIndexImpl>,
    pub index: Arc<FullTextIndexImpl>,
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
            g.as_ref()
                .map(|ft| Arc::clone(&ft.index) as Arc<dyn FullTextIndex>)
        });
        (vault, fts)
    }
}

pub struct AppState {
    pub record_note_opened_use_case: RecordNoteOpenedUseCase<WorkspaceSessionRepositoryImpl>,
    pub get_recent_notes_use_case: GetRecentNotesUseCase<WorkspaceSessionRepositoryImpl>,
    pub save_open_tabs_use_case: SaveOpenTabsUseCase<WorkspaceSessionRepositoryImpl>,
    pub get_open_tabs_use_case: GetOpenTabsUseCase<WorkspaceSessionRepositoryImpl>,
    pub set_note_view_mode_use_case: SetNoteViewModeUseCase<WorkspaceSessionRepositoryImpl>,
    pub get_note_view_mode_use_case: GetNoteViewModeUseCase<WorkspaceSessionRepositoryImpl>,
    pub save_vault_ui_state_use_case: SaveVaultUiStateUseCase<WorkspaceSessionRepositoryImpl>,
    pub get_vault_ui_state_use_case: GetVaultUiStateUseCase<WorkspaceSessionRepositoryImpl>,
    pub delete_vault_item_use_case: DeleteVaultItemUseCase<VaultItemRepositoryImpl>,
    pub rename_vault_item_use_case:
        RenameVaultItemUseCase<VaultItemRepositoryImpl, VersionControlImpl>,
    pub move_vault_items_use_case:
        MoveVaultItemsUseCase<VaultItemRepositoryImpl, VersionControlImpl>,
    pub copy_vault_items_use_case: CopyVaultItemsUseCase<VaultItemRepositoryImpl>,
    pub read_binary_vault_item_use_case: ReadBinaryVaultItemUseCase<VaultItemRepositoryImpl>,
    pub write_binary_vault_item_use_case: WriteBinaryVaultItemUseCase<VaultItemRepositoryImpl>,
    pub create_folder_use_case: CreateFolderUseCase<VaultItemRepositoryImpl>,
    pub get_vault_version_state_use_case: GetVaultVersionStateUseCase<VersionControlImpl>,
    pub track_files_use_case: TrackFilesUseCase<VersionControlImpl>,
    pub revert_files_use_case: RevertFilesUseCase<VersionControlImpl>,
    pub untrack_files_use_case: UntrackFilesUseCase<VersionControlImpl>,
    pub save_version_use_case: SaveVersionUseCase<VersionControlImpl>,
    pub get_file_diff_use_case: GetFileDiffUseCase<VersionControlImpl>,
    pub get_file_revision_use_case: GetFileRevisionUseCase<VersionControlImpl>,
    pub get_image_diff_use_case: GetImageDiffUseCase<VersionControlImpl>,
    pub get_file_version_history_use_case: GetFileVersionHistoryUseCase<VersionControlImpl>,
    pub get_branches_use_case: GetBranchesUseCase<VersionControlImpl>,
    pub checkout_branch_use_case: CheckoutBranchUseCase<VersionControlImpl>,
    pub create_branch_use_case: CreateBranchUseCase<VersionControlImpl>,
    pub clone_repository_use_case: CloneRepositoryUseCase<VersionControlImpl>,
    pub push_versions_use_case: PushVersionsUseCase<VersionControlImpl>,
    pub open_file_with_default_app_use_case: OpenFileWithDefaultAppUseCase<SystemShellImpl>,
    pub show_item_in_folder_use_case: ShowItemInFolderUseCase<SystemShellImpl>,
    pub active_vault_path: Arc<Mutex<Option<PathBuf>>>,
    pub file_types: SupportedFileTypes,
    pub list_notes_use_case: ListNotesUseCase<NoteRepositoryImpl>,
    pub read_note_use_case: ReadNoteUseCase<NoteRepositoryImpl>,
    pub save_note_use_case: SaveNoteUseCase<NoteRepositoryImpl>,
    pub read_standalone_note_use_case: ReadStandaloneNoteUseCase<NoteRepositoryImpl>,
    pub save_standalone_note_use_case: SaveStandaloneNoteUseCase<NoteRepositoryImpl>,
    pub get_workspace_registry_use_case:
        GetWorkspaceRegistryUseCase<WorkspaceRegistryRepositoryImpl>,
    pub list_workspaces_use_case: ListWorkspacesUseCase<WorkspaceRegistryRepositoryImpl>,
    pub record_workspace_opened_use_case:
        RecordWorkspaceOpenedUseCase<WorkspaceRegistryRepositoryImpl>,
    pub toggle_workspace_pin_use_case: ToggleWorkspacePinUseCase<WorkspaceRegistryRepositoryImpl>,
    pub remove_workspace_use_case: RemoveWorkspaceUseCase<WorkspaceRegistryRepositoryImpl>,
    pub get_shortcuts_config_use_case: GetShortcutsConfigUseCase<WorkspaceRegistryRepositoryImpl>,
    pub save_shortcut_use_case: SaveShortcutUseCase<WorkspaceRegistryRepositoryImpl>,
    pub fetch_community_plugins_use_case:
        FetchCommunityPluginsUseCase<CommunityPluginDownloaderImpl>,
    pub get_installed_plugins_use_case: GetInstalledPluginsUseCase<InstalledPluginRepositoryImpl>,
    pub check_plugin_updates_use_case: CheckPluginUpdatesUseCase<CommunityPluginDownloaderImpl>,
    pub install_plugin_use_case: InstallPluginUseCase<CommunityPluginDownloaderImpl>,
    pub uninstall_plugin_use_case: UninstallPluginUseCase<CommunityPluginDownloaderImpl>,
    pub navigation_engine: Arc<Mutex<Option<Arc<NavigationEngine>>>>,
    pub full_text: Arc<Mutex<Option<Arc<FullTextComponents>>>>,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    pub detached_tabs: Arc<Mutex<HashMap<String, String>>>,
    pub startup_target: Arc<Mutex<Option<StartupTarget>>>,
    pub mcp_runtime: Arc<McpBackgroundRuntime>,
    /// Proporción (porcentaje) del panel de código en los visores de diagramas, por archivo. Solo en memoria.
    pub diagram_split_ratios: Arc<DashMap<String, f32>>,
}

pub fn open_vault_components(
    vault_path: PathBuf,
    file_types: SupportedFileTypes,
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
) -> (Arc<NavigationEngine>, Arc<FullTextComponents>) {
    let cache_path = vault_path.join(".synapse").join("cache.bin");
    let engine =
        NavigationEngine::start_with_file_types(vault_path.clone(), cache_path, file_types.clone());

    let handle_for_fs = Arc::clone(&app_handle);
    if let Ok(guard) = app_handle.lock() {
        if let Some(ref handle) = *guard {
            use tauri::Manager;
            let _ = handle
                .asset_protocol_scope()
                .allow_directory(&vault_path, true);
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
    let tantivy_index = match FullTextIndexImpl::open_or_create(&fts_dir) {
        Ok(idx) => Arc::new(idx),
        Err(e) => {
            log::warn!(
                "No se pudo abrir índice FTS en disco ({}), usando fallback en RAM",
                e
            );
            Arc::new(FullTextIndexImpl::in_memory().expect("Fallo al crear índice FTS en RAM"))
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
    let index_vault_change = IndexVaultChangeUseCase::new(Arc::clone(&indexer));
    engine.register_change_listener(Arc::new(move |upserted, removed| {
        let _ = index_vault_change.execute(IndexVaultChangeCommand { upserted, removed });
    }));

    let search_use_case = SearchFullTextUseCase::new(Arc::clone(&tantivy_index));
    let full_text = Arc::new(FullTextComponents {
        search_use_case,
        index: tantivy_index,
        indexer,
    });

    (engine, full_text)
}

impl AppState {
    pub fn empty(file_types: SupportedFileTypes, note_repository: NoteRepositoryImpl) -> Self {
        let workspace_registry_repository =
            WorkspaceRegistryRepositoryImpl::new(file_types.clone());
        let community_plugin_downloader = CommunityPluginDownloaderImpl::new();
        // Ensure ~/.config/synapsers/workspaces.json and shortcuts.jsonc exist on startup
        let _ = GetWorkspaceRegistryUseCase::new(workspace_registry_repository.clone())
            .execute(GetWorkspaceRegistryQuery {});
        let _ = GetShortcutsConfigUseCase::new(workspace_registry_repository.clone())
            .execute(GetShortcutsConfigQuery {});

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
            let handle_clone = Arc::clone(&handle_for_mcp);
            std::thread::spawn(move || {
                if let Ok(guard) = handle_clone.lock() {
                    if let Some(ref handle) = *guard {
                        use tauri::Emitter;
                        let _ = handle.emit("mcp:status-changed", &status);
                    }
                }
            });
        }));

        let navigation_engine = Arc::new(Mutex::new(None));
        let workspace_session_repository =
            WorkspaceSessionRepositoryImpl::new(Arc::clone(&navigation_engine));

        Self {
            active_vault_path,
            file_types,
            list_notes_use_case: ListNotesUseCase::new(note_repository.clone()),
            read_note_use_case: ReadNoteUseCase::new(note_repository.clone()),
            save_note_use_case: SaveNoteUseCase::new(note_repository.clone()),
            read_standalone_note_use_case: ReadStandaloneNoteUseCase::new(note_repository.clone()),
            save_standalone_note_use_case: SaveStandaloneNoteUseCase::new(note_repository.clone()),
            get_workspace_registry_use_case: GetWorkspaceRegistryUseCase::new(
                workspace_registry_repository.clone(),
            ),
            list_workspaces_use_case: ListWorkspacesUseCase::new(
                workspace_registry_repository.clone(),
            ),
            record_workspace_opened_use_case: RecordWorkspaceOpenedUseCase::new(
                workspace_registry_repository.clone(),
            ),
            toggle_workspace_pin_use_case: ToggleWorkspacePinUseCase::new(
                workspace_registry_repository.clone(),
            ),
            remove_workspace_use_case: RemoveWorkspaceUseCase::new(
                workspace_registry_repository.clone(),
            ),
            get_shortcuts_config_use_case: GetShortcutsConfigUseCase::new(
                workspace_registry_repository.clone(),
            ),
            save_shortcut_use_case: SaveShortcutUseCase::new(
                workspace_registry_repository.clone(),
            ),
            fetch_community_plugins_use_case: FetchCommunityPluginsUseCase::new(
                community_plugin_downloader.clone(),
            ),
            get_installed_plugins_use_case: GetInstalledPluginsUseCase::new(
                InstalledPluginRepositoryImpl,
            ),
            check_plugin_updates_use_case: CheckPluginUpdatesUseCase::new(
                community_plugin_downloader.clone(),
            ),
            install_plugin_use_case: InstallPluginUseCase::new(community_plugin_downloader.clone()),
            uninstall_plugin_use_case: UninstallPluginUseCase::new(
                community_plugin_downloader.clone(),
            ),
            delete_vault_item_use_case: DeleteVaultItemUseCase::new(VaultItemRepositoryImpl),
            rename_vault_item_use_case: RenameVaultItemUseCase::new(
                VaultItemRepositoryImpl,
                VersionControlImpl,
            ),
            move_vault_items_use_case: MoveVaultItemsUseCase::new(
                VaultItemRepositoryImpl,
                VersionControlImpl,
            ),
            copy_vault_items_use_case: CopyVaultItemsUseCase::new(VaultItemRepositoryImpl),
            read_binary_vault_item_use_case: ReadBinaryVaultItemUseCase::new(
                VaultItemRepositoryImpl,
            ),
            write_binary_vault_item_use_case: WriteBinaryVaultItemUseCase::new(
                VaultItemRepositoryImpl,
            ),
            create_folder_use_case: CreateFolderUseCase::new(VaultItemRepositoryImpl),
            get_vault_version_state_use_case: GetVaultVersionStateUseCase::new(VersionControlImpl),
            track_files_use_case: TrackFilesUseCase::new(VersionControlImpl),
            revert_files_use_case: RevertFilesUseCase::new(VersionControlImpl),
            untrack_files_use_case: UntrackFilesUseCase::new(VersionControlImpl),
            save_version_use_case: SaveVersionUseCase::new(VersionControlImpl),
            get_file_diff_use_case: GetFileDiffUseCase::new(VersionControlImpl),
            get_file_revision_use_case: GetFileRevisionUseCase::new(VersionControlImpl),
            get_image_diff_use_case: GetImageDiffUseCase::new(VersionControlImpl),
            get_file_version_history_use_case: GetFileVersionHistoryUseCase::new(
                VersionControlImpl,
            ),
            get_branches_use_case: GetBranchesUseCase::new(VersionControlImpl),
            checkout_branch_use_case: CheckoutBranchUseCase::new(VersionControlImpl),
            create_branch_use_case: CreateBranchUseCase::new(VersionControlImpl),
            clone_repository_use_case: CloneRepositoryUseCase::new(VersionControlImpl),
            push_versions_use_case: PushVersionsUseCase::new(VersionControlImpl),
            open_file_with_default_app_use_case: OpenFileWithDefaultAppUseCase::new(
                SystemShellImpl,
            ),
            show_item_in_folder_use_case: ShowItemInFolderUseCase::new(SystemShellImpl),
            record_note_opened_use_case: RecordNoteOpenedUseCase::new(
                workspace_session_repository.clone(),
            ),
            get_recent_notes_use_case: GetRecentNotesUseCase::new(
                workspace_session_repository.clone(),
            ),
            save_open_tabs_use_case: SaveOpenTabsUseCase::new(workspace_session_repository.clone()),
            get_open_tabs_use_case: GetOpenTabsUseCase::new(workspace_session_repository.clone()),
            set_note_view_mode_use_case: SetNoteViewModeUseCase::new(
                workspace_session_repository.clone(),
            ),
            get_note_view_mode_use_case: GetNoteViewModeUseCase::new(
                workspace_session_repository.clone(),
            ),
            save_vault_ui_state_use_case: SaveVaultUiStateUseCase::new(
                workspace_session_repository.clone(),
            ),
            get_vault_ui_state_use_case: GetVaultUiStateUseCase::new(
                workspace_session_repository.clone(),
            ),
            navigation_engine,
            full_text,
            app_handle,
            detached_tabs: Arc::new(Mutex::new(HashMap::new())),
            startup_target: Arc::new(Mutex::new(None)),
            mcp_runtime,
            diagram_split_ratios: Arc::new(DashMap::new()),
        }
    }

    pub fn new(
        initial_vault_path: PathBuf,
        file_types: SupportedFileTypes,
        note_repository: NoteRepositoryImpl,
    ) -> Self {
        let app_handle: Arc<Mutex<Option<tauri::AppHandle>>> = Arc::new(Mutex::new(None));
        let workspace_registry_repository =
            WorkspaceRegistryRepositoryImpl::new(file_types.clone());
        let community_plugin_downloader = CommunityPluginDownloaderImpl::new();
        let (engine, full_text_comps) = open_vault_components(
            initial_vault_path.clone(),
            file_types.clone(),
            Arc::clone(&app_handle),
        );

        let active_vault_path = Arc::new(Mutex::new(Some(initial_vault_path.clone())));
        let full_text = Arc::new(Mutex::new(Some(full_text_comps)));

        let mcp_provider = Arc::new(DynamicMcpContextProvider {
            active_vault_path: Arc::clone(&active_vault_path),
            full_text: Arc::clone(&full_text),
        });
        let mcp_runtime = Arc::new(McpBackgroundRuntime::new(mcp_provider));

        let handle_for_mcp = Arc::clone(&app_handle);
        mcp_runtime.set_on_status_change(Arc::new(move |status| {
            let handle_clone = Arc::clone(&handle_for_mcp);
            std::thread::spawn(move || {
                if let Ok(guard) = handle_clone.lock() {
                    if let Some(ref handle) = *guard {
                        use tauri::Emitter;
                        let _ = handle.emit("mcp:status-changed", &status);
                    }
                }
            });
        }));

        mcp_runtime.sync_for_vault(&initial_vault_path);

        let navigation_engine = Arc::new(Mutex::new(Some(engine)));
        let workspace_session_repository =
            WorkspaceSessionRepositoryImpl::new(Arc::clone(&navigation_engine));

        Self {
            active_vault_path,
            file_types,
            list_notes_use_case: ListNotesUseCase::new(note_repository.clone()),
            read_note_use_case: ReadNoteUseCase::new(note_repository.clone()),
            save_note_use_case: SaveNoteUseCase::new(note_repository.clone()),
            read_standalone_note_use_case: ReadStandaloneNoteUseCase::new(note_repository.clone()),
            save_standalone_note_use_case: SaveStandaloneNoteUseCase::new(note_repository.clone()),
            get_workspace_registry_use_case: GetWorkspaceRegistryUseCase::new(
                workspace_registry_repository.clone(),
            ),
            list_workspaces_use_case: ListWorkspacesUseCase::new(
                workspace_registry_repository.clone(),
            ),
            record_workspace_opened_use_case: RecordWorkspaceOpenedUseCase::new(
                workspace_registry_repository.clone(),
            ),
            toggle_workspace_pin_use_case: ToggleWorkspacePinUseCase::new(
                workspace_registry_repository.clone(),
            ),
            remove_workspace_use_case: RemoveWorkspaceUseCase::new(
                workspace_registry_repository.clone(),
            ),
            get_shortcuts_config_use_case: GetShortcutsConfigUseCase::new(
                workspace_registry_repository.clone(),
            ),
            save_shortcut_use_case: SaveShortcutUseCase::new(
                workspace_registry_repository.clone(),
            ),
            fetch_community_plugins_use_case: FetchCommunityPluginsUseCase::new(
                community_plugin_downloader.clone(),
            ),
            get_installed_plugins_use_case: GetInstalledPluginsUseCase::new(
                InstalledPluginRepositoryImpl,
            ),
            check_plugin_updates_use_case: CheckPluginUpdatesUseCase::new(
                community_plugin_downloader.clone(),
            ),
            install_plugin_use_case: InstallPluginUseCase::new(community_plugin_downloader.clone()),
            uninstall_plugin_use_case: UninstallPluginUseCase::new(
                community_plugin_downloader.clone(),
            ),
            delete_vault_item_use_case: DeleteVaultItemUseCase::new(VaultItemRepositoryImpl),
            rename_vault_item_use_case: RenameVaultItemUseCase::new(
                VaultItemRepositoryImpl,
                VersionControlImpl,
            ),
            move_vault_items_use_case: MoveVaultItemsUseCase::new(
                VaultItemRepositoryImpl,
                VersionControlImpl,
            ),
            copy_vault_items_use_case: CopyVaultItemsUseCase::new(VaultItemRepositoryImpl),
            read_binary_vault_item_use_case: ReadBinaryVaultItemUseCase::new(
                VaultItemRepositoryImpl,
            ),
            write_binary_vault_item_use_case: WriteBinaryVaultItemUseCase::new(
                VaultItemRepositoryImpl,
            ),
            create_folder_use_case: CreateFolderUseCase::new(VaultItemRepositoryImpl),
            get_vault_version_state_use_case: GetVaultVersionStateUseCase::new(VersionControlImpl),
            track_files_use_case: TrackFilesUseCase::new(VersionControlImpl),
            revert_files_use_case: RevertFilesUseCase::new(VersionControlImpl),
            untrack_files_use_case: UntrackFilesUseCase::new(VersionControlImpl),
            save_version_use_case: SaveVersionUseCase::new(VersionControlImpl),
            get_file_diff_use_case: GetFileDiffUseCase::new(VersionControlImpl),
            get_file_revision_use_case: GetFileRevisionUseCase::new(VersionControlImpl),
            get_image_diff_use_case: GetImageDiffUseCase::new(VersionControlImpl),
            get_file_version_history_use_case: GetFileVersionHistoryUseCase::new(
                VersionControlImpl,
            ),
            get_branches_use_case: GetBranchesUseCase::new(VersionControlImpl),
            checkout_branch_use_case: CheckoutBranchUseCase::new(VersionControlImpl),
            create_branch_use_case: CreateBranchUseCase::new(VersionControlImpl),
            clone_repository_use_case: CloneRepositoryUseCase::new(VersionControlImpl),
            push_versions_use_case: PushVersionsUseCase::new(VersionControlImpl),
            open_file_with_default_app_use_case: OpenFileWithDefaultAppUseCase::new(
                SystemShellImpl,
            ),
            show_item_in_folder_use_case: ShowItemInFolderUseCase::new(SystemShellImpl),
            record_note_opened_use_case: RecordNoteOpenedUseCase::new(
                workspace_session_repository.clone(),
            ),
            get_recent_notes_use_case: GetRecentNotesUseCase::new(
                workspace_session_repository.clone(),
            ),
            save_open_tabs_use_case: SaveOpenTabsUseCase::new(workspace_session_repository.clone()),
            get_open_tabs_use_case: GetOpenTabsUseCase::new(workspace_session_repository.clone()),
            set_note_view_mode_use_case: SetNoteViewModeUseCase::new(
                workspace_session_repository.clone(),
            ),
            get_note_view_mode_use_case: GetNoteViewModeUseCase::new(
                workspace_session_repository.clone(),
            ),
            save_vault_ui_state_use_case: SaveVaultUiStateUseCase::new(
                workspace_session_repository.clone(),
            ),
            get_vault_ui_state_use_case: GetVaultUiStateUseCase::new(
                workspace_session_repository.clone(),
            ),
            navigation_engine,
            full_text,
            app_handle,
            detached_tabs: Arc::new(Mutex::new(HashMap::new())),
            startup_target: Arc::new(Mutex::new(None)),
            mcp_runtime,
            diagram_split_ratios: Arc::new(DashMap::new()),
        }
    }

    pub fn activate_vault(
        &self,
        path: PathBuf,
        is_git: Option<bool>,
    ) -> Result<SelectVaultFolderResult, String> {
        let folder_path_str = path.to_string_lossy().to_string();

        {
            let mut vault_path = self.active_vault_path.lock().map_err(|e| e.to_string())?;
            *vault_path = Some(path.clone());
        }

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

        let notes = VaultEntryDto::list(&path, &new_engine, false);

        // Sincronizar servidor MCP con la configuración específica de esta bóveda
        self.mcp_runtime.sync_for_vault(&path);

        let _ = self
            .record_workspace_opened_use_case
            .execute(RecordWorkspaceOpenedCommand {
                path: folder_path_str.clone(),
                notes_count: Some(notes.len()),
                git_branch: None,
                is_git_repo: is_git,
            });

        Ok(SelectVaultFolderResult {
            folder_path: folder_path_str,
            notes,
        })
    }
}
