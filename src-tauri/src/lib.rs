pub mod application;
pub mod domain;
pub mod infrastructure;

use application::use_cases::note_use_cases::NoteUseCases;
use domain::models::file_types::SupportedFileTypes;
use infrastructure::repositories::file_note_repository::FileNoteRepository;
use infrastructure::tauri::commands::{
    copy_vault_items, delete_vault_item, full_text_search, get_active_vault_path, get_full_text_index_status,
    get_open_tabs_state, get_recent_notes_command, get_supported_file_types, get_system_theme,
    get_vault_directory_children, get_vault_files, get_vault_files_count, get_vault_git_status, get_vault_notes,
    get_note_meta, get_vault_ui_state, git_add_paths, git_commit_paths, git_restore_paths, git_restore_staged_paths,
    get_git_file_diff, get_git_file_revision, get_git_image_diff, get_git_file_log, get_git_branches, git_checkout_branch, git_create_branch,
    read_asset_data_url, read_binary_file, write_binary_file, create_vault_folder, read_note_content, rebuild_full_text_index, record_note_opened, reload_vault_items, rename_vault_item,
    render_markdown_wikilinks, resolve_asset_file_path, resolve_vault_link, resolve_vault_links, save_binary_asset, save_note_content,
    save_open_tabs_state, save_vault_ui_state, search_items_command, search_notes_command, select_vault_folder,
    set_active_vault_path, set_note_view_mode, toggle_devtools, open_tab_in_new_window, get_detached_window_tab,
    get_recent_workspaces, get_shortcuts_config, toggle_pin_workspace, remove_recent_workspace, create_new_workspace, clone_git_workspace, open_workspace_in_new_window,
    open_file_with_default_app, show_item_in_folder, get_startup_target, select_standalone_file, parse_startup_args,
    fetch_community_plugins, install_community_plugin, uninstall_community_plugin,
    get_installed_community_plugins, check_community_plugin_updates,
    check_vault_path_exists, list_vault_directory,
    AppState,
};
use std::env;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
//use webkit2gtk_nvidia_quirk::ApplyWorkaroundOptions;
#[cfg(target_os = "linux")]
use webkit2gtk_nvidia_quirk::{apply_workaround_with_options, ApplyWorkaroundOptions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let file_types = SupportedFileTypes::default();
    let repo = FileNoteRepository::new();
    let use_cases = NoteUseCases::new(repo);
    let app_state = AppState::empty(file_types, use_cases);

    let startup_target = parse_startup_args(&std::env::args().collect::<Vec<_>>());
    if let Ok(mut st) = app_state.startup_target.lock() {
        *st = startup_target;
    }

    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        let options = ApplyWorkaroundOptions::default().force_disable_nv_explicit_sync(true);
        apply_workaround_with_options(options);
    }

    let mut builder = tauri::Builder::default();

    if cfg!(debug_assertions) {
        builder = builder.plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .level(log::LevelFilter::Debug)
                .build(),
        );
    }

    builder
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_supported_file_types,
            get_vault_notes,
            get_vault_files,
            get_note_meta,
            get_vault_files_count,
            get_vault_directory_children,
            get_active_vault_path,
            get_open_tabs_state,
            get_recent_notes_command,
            read_note_content,
            record_note_opened,
            save_note_content,
            save_open_tabs_state,
            save_vault_ui_state,
            get_vault_ui_state,
            set_active_vault_path,
            set_note_view_mode,
            select_vault_folder,
            search_items_command,
            search_notes_command,
            toggle_devtools,
            get_vault_git_status,
            delete_vault_item,
            rename_vault_item,
            copy_vault_items,
            git_add_paths,
            git_commit_paths,
            git_restore_paths,
            git_restore_staged_paths,
            get_git_file_diff,
            get_git_file_revision,
            get_git_image_diff,
            get_git_file_log,
            get_git_branches,
            git_checkout_branch,
            git_create_branch,
            reload_vault_items,
            get_system_theme,
            full_text_search,
            get_full_text_index_status,
            rebuild_full_text_index,
            resolve_vault_link,
            resolve_vault_links,
            render_markdown_wikilinks,
            resolve_asset_file_path,
            read_asset_data_url,
            read_binary_file,
            write_binary_file,
            create_vault_folder,
            save_binary_asset,
            open_tab_in_new_window,
            get_detached_window_tab,
            get_recent_workspaces,
            get_shortcuts_config,
            toggle_pin_workspace,
            remove_recent_workspace,
            create_new_workspace,
            clone_git_workspace,
            open_workspace_in_new_window,
            open_file_with_default_app,
            show_item_in_folder,
            get_startup_target,
            select_standalone_file,
            fetch_community_plugins,
            install_community_plugin,
            uninstall_community_plugin,
            get_installed_community_plugins,
            check_community_plugin_updates,
            check_vault_path_exists,
            list_vault_directory
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            if let Ok(mut h) = state.app_handle.lock() {
                *h = Some(handle.clone());
            }
            if let Ok(guard) = state.active_vault_path.lock() {
                if let Some(ref vault_path) = *guard {
                    let _ = handle.asset_protocol_scope().allow_directory(vault_path, true);
                }
            }
            if let Ok(guard) = state.startup_target.lock() {
                if let Some(domain::models::startup::StartupTarget::Standalone(ref path_str)) = *guard {
                    let p = std::path::Path::new(path_str);
                    if let Some(parent) = p.parent() {
                        let _ = handle.asset_protocol_scope().allow_directory(parent, true);
                    }
                }
            }
            if let Ok(guard) = state.navigation_engine.lock() {
                if let Some(ref engine) = *guard {
                    let h = handle.clone();
                    engine.set_on_fs_change(std::sync::Arc::new(move |evt| {
                        use tauri::Emitter;
                        let _ = h.emit("vault:files-changed", &evt);
                    }));
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
