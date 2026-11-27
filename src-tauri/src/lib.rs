pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod version;

use domain::vault::file_types::SupportedFileTypes;
use infrastructure::entrypoint::tauri::commands::{
    AppState, check_community_plugin_updates, check_vault_path_exists, clone_git_workspace,
    copy_vault_items, is_drawio_image, import_external_items, copy_files_to_system_clipboard, read_system_clipboard_files, create_new_workspace, create_vault_folder, delete_vault_item,
    diff_git_revisions, diff_vault_files, docx_bytes, docx_diff, docx_image, fetch_community_plugins, fetch_plantuml_svg,
    board_convert, board_read, board_task_delete, board_task_read, board_task_write, board_write,
    issues_activity, issues_add_comment, issues_check_connection, issues_connections, issues_create, issues_delete_connection, issues_fetch, issues_filter_query, issues_filter_values, issues_providers, issues_save_connection, issues_update, issues_update_comment,
    get_azure_token_setting, set_azure_token,
    full_text_search, get_active_vault_path, get_app_version, get_default_theme_dir,
    get_detached_window_tab, get_diagram_split_ratio, get_full_text_index_status, get_git_branches, get_git_commit_details, get_git_repo_log, git_change_history, git_conflict_versions, git_merge, git_merge_state, git_create_tag, git_delete_ref, git_fast_forward, git_squash_preview, spreadsheet_open, spreadsheet_layout, spreadsheet_cells, spreadsheet_find,
    get_git_file_diff, get_git_file_log, get_git_file_revision, get_git_image_diff,
    get_installed_community_plugins, get_mcp_port_registry, get_mcp_server_status,
    get_note_backlinks_and_relations, get_note_meta, get_notes_by_tag, get_open_tabs_state,
    get_recent_notes_command, get_recent_workspaces, get_shortcuts_config, get_startup_target,
    get_supported_file_types, get_system_theme, get_theme_dirs, get_vault_directory_children,
    get_vault_files, get_vault_files_count, get_vault_git_status, get_vault_graph, get_vault_notes,
    get_vault_tags, get_vault_ui_state, git_add_paths, git_checkout_branch, git_commit_paths,
    git_create_branch, git_push_vault, git_restore_paths, git_restore_staged_paths,
    install_community_plugin, list_global_themes, list_vault_directory, mcp_confirm_reply,
    mcp_conversations_reply, mcp_diagram_rendered, move_vault_items, open_file_with_default_app,
    open_tab_in_new_window, open_workspace_in_new_window, parse_startup_args, peer_identity,
    peer_list, peer_send, pick_image_file, notify_if_unfocused, request_attention, read_asset_data_url, read_binary_file,
    read_note_content, rebuild_full_text_index, record_note_opened, reload_vault_items,
    remove_recent_workspace, rename_vault_item, render_markdown_wikilinks, reorder_pinned_workspaces, resolve_asset_file_path,
    resolve_vault_link, resolve_vault_links, resolve_vault_link_detailed, save_binary_asset, save_note_content,
    save_open_tabs_state, save_shortcut, save_theme_dirs, save_vault_ui_state,
    search_folders_command, search_items_command, search_notes_command, select_standalone_file,
    select_vault_folder, set_active_vault_path, set_diagram_split_ratio, set_note_view_mode,
    show_item_in_folder, start_mcp_server, stop_mcp_server, toggle_devtools, toggle_mcp_server,
    toggle_pin_workspace, uninstall_community_plugin, write_binary_file,
    check_for_update, install_update, restart_app,
    get_git_info, set_git_path, pick_git_executable,
};
use infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use std::env;
use tauri::Manager;
#[cfg(feature = "desktop")]
use tauri_plugin_cli::CliExt;
use tauri_plugin_log::{Target, TargetKind};
//use webkit2gtk_nvidia_quirk::ApplyWorkaroundOptions;
#[cfg(all(target_os = "linux", feature = "desktop"))]
use webkit2gtk_nvidia_quirk::{ApplyWorkaroundOptions, apply_workaround_with_options};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let file_types = SupportedFileTypes::default();
    let app_state = AppState::empty(file_types, NoteRepositoryImpl::new());

    let startup_target = parse_startup_args(&std::env::args().collect::<Vec<_>>());
    if let Ok(mut st) = app_state.startup_target.lock() {
        *st = startup_target;
    }

    #[cfg(all(target_os = "linux", feature = "desktop"))]
    {
        // El renderizador DMABUF de WebKitGTK es el rápido (draw.io, por ejemplo, va mucho más fluido)
        // y queda activo. Si con algún driver deja la ventana en blanco o parpadea, `SYNAPSE_DMABUF=0`
        // lo apaga y vuelve al modo seguro y lento.
        if std::env::var("SYNAPSE_DMABUF").is_ok_and(|v| v == "0") {
            // SAFETY: se ejecuta al inicio de `run`, antes de crear hilos propios ni inicializar WebKit
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
        let options = ApplyWorkaroundOptions::default().force_disable_nv_explicit_sync(true);
        apply_workaround_with_options(options);
    }

    let mut builder = tauri::Builder::default();

    #[cfg(feature = "desktop")]
    {
        builder = builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_cli::init());
    }

    builder = builder.plugin(tauri_plugin_notification::init());

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
            check_for_update,
            install_update,
            restart_app,
            get_git_info,
            set_git_path,
            pick_git_executable,
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
            get_diagram_split_ratio,
            set_diagram_split_ratio,
            select_vault_folder,
            search_items_command,
            search_notes_command,
            search_folders_command,
            toggle_devtools,
            fetch_plantuml_svg,
            board_convert,
            board_read,
            board_write,
            board_task_read,
            board_task_write,
            board_task_delete,
            issues_providers,
            issues_connections,
            issues_save_connection,
            issues_delete_connection,
            issues_check_connection,
            issues_fetch,
            issues_filter_values,
            issues_filter_query,
            get_azure_token_setting,
            set_azure_token,
            issues_update,
            issues_update_comment,
            issues_create,
            issues_activity,
            issues_add_comment,
            get_vault_git_status,
            delete_vault_item,
            rename_vault_item,
            copy_vault_items,
            is_drawio_image,
            request_attention,
            notify_if_unfocused,
            import_external_items,
            copy_files_to_system_clipboard,
            read_system_clipboard_files,
            move_vault_items,
            git_add_paths,
            git_commit_paths,
            git_restore_paths,
            git_restore_staged_paths,
            get_git_file_diff,
            diff_vault_files,
            diff_git_revisions,
            docx_bytes,
            docx_diff,
            docx_image,
            get_git_file_revision,
            get_git_image_diff,
            get_git_file_log,
            get_git_branches,
            get_git_repo_log,
            get_git_commit_details,
            git_create_tag,
            git_fast_forward,
            git_squash_preview,
            git_change_history,
            git_merge_state,
            git_conflict_versions,
            git_merge,
            spreadsheet_open,
            spreadsheet_layout,
            spreadsheet_cells,
            spreadsheet_find,
            git_delete_ref,
            git_checkout_branch,
            git_create_branch,
            git_push_vault,
            reload_vault_items,
            get_system_theme,
            full_text_search,
            get_full_text_index_status,
            rebuild_full_text_index,
            resolve_vault_link,
            resolve_vault_links, resolve_vault_link_detailed,
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
            save_shortcut,
            get_default_theme_dir,
            get_theme_dirs,
            save_theme_dirs,
            list_global_themes,
            pick_image_file,
            toggle_pin_workspace,
            reorder_pinned_workspaces,
            remove_recent_workspace,
            create_new_workspace,
            clone_git_workspace,
            open_workspace_in_new_window,
            open_file_with_default_app,
            get_app_version,
            show_item_in_folder,
            get_startup_target,
            select_standalone_file,
            fetch_community_plugins,
            install_community_plugin,
            uninstall_community_plugin,
            get_installed_community_plugins,
            check_community_plugin_updates,
            check_vault_path_exists,
            list_vault_directory,
            get_vault_graph,
            get_vault_tags,
            get_notes_by_tag,
            get_note_backlinks_and_relations,
            get_mcp_server_status,
            mcp_diagram_rendered,
            mcp_conversations_reply,
            mcp_confirm_reply,
            peer_list,
            peer_send,
            peer_identity,
            get_mcp_port_registry,
            toggle_mcp_server,
            start_mcp_server,
            stop_mcp_server,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_list,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_install,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_save_profile,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_delete_profile,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_start_session,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_send_prompt,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_quota,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_cancel_prompt,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_respond_permission,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_respond_elicitation,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_stop_session,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_set_mode,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_set_title,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_session_title,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_set_model,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_set_option,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_save_transcript,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_list_transcripts,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_load_transcript,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_delete_transcript,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_delete_all_transcripts,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_feedback_available,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_set_debug_log,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_set_process_limit,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_open_transcript,
            infrastructure::entrypoint::controller::tauri::agent_controller::agent_record_iteration
        ])
        .setup(|app| {
            // Restos de una actualización anterior (Windows deja el .exe viejo como .old)
            infrastructure::service::os::self_update::cleanup_old_binary();
            // Ruta de Git elegida por el usuario (config.jsonc → git.path)
            infrastructure::service::os::git_binary::load(
                &infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl::resolve_config_dir(),
            );
            #[cfg(feature = "desktop")]
            if let Ok(matches) = app.cli().matches() {
                if let Some(help) = matches.args.get("help").and_then(|a| a.value.as_str()) {
                    print!("{}", help);
                    std::process::exit(0);
                }
                if let Some(arg) = matches.args.get("version") {
                    if let Some(version) = arg.value.as_str() {
                        println!("{}", version);
                        std::process::exit(0);
                    } else {
                        // Cuando se invoca con -V o --version, clap pasa version con val Null
                        println!("synapsers {}", *version::VERSION);
                        std::process::exit(0);
                    }
                }
            }

            let handle = app.handle().clone();
            {
                // Agentes externos: sus eventos van a la interfaz
                use tauri::Emitter;
                let h = handle.clone();
                app.manage(infrastructure::config::agent_state::AgentState::new(
                    std::sync::Arc::new(move |name: &str, payload: serde_json::Value| {
                        let _ = h.emit(name, payload);
                    }),
                ));
            }
            let state = app.state::<AppState>();
            if let Ok(mut h) = state.app_handle.lock() {
                *h = Some(handle.clone());
            }
            if let Ok(guard) = state.active_vault_path.lock()
                && let Some(ref vault_path) = *guard {
                    let _ = handle
                        .asset_protocol_scope()
                        .allow_directory(vault_path, true);
                }
            if let Ok(guard) = state.startup_target.lock() {
                if let Some(domain::config::startup::StartupTarget::Standalone(ref path_str)) =
                    *guard
                {
                    state.grant_file(std::path::Path::new(path_str));
                }
                if let Some(domain::config::startup::StartupTarget::Vault(ref path_str)) = *guard {
                    state.allowed_paths.grant_vault(std::path::Path::new(path_str));
                }
            }
            for wallpaper in infrastructure::config::allowed_paths::load_wallpapers(
                &infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl::resolve_config_dir(),
            ) {
                state.allow_asset(&wallpaper);
            }
            if let Ok(guard) = state.navigation_engine.lock()
                && let Some(ref engine) = *guard {
                    let h = handle.clone();
                    engine.set_on_fs_change(std::sync::Arc::new(move |evt| {
                        use tauri::Emitter;
                        let _ = h.emit("vault:files-changed", &evt);
                    }));
                }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Solo el sistema emite este evento: la interfaz no puede fingir qué se soltó
            if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                window.state::<AppState>().allowed_paths.set_dropped(paths);
            }
            if window.label() == "main"
                && let tauri::WindowEvent::Destroyed = event {
                    let app = window.app_handle();
                    let state = app.state::<AppState>();
                    let _ = state.mcp_runtime.stop();
                    for (label, other_win) in app.webview_windows() {
                        if label != "main" {
                            let _ = other_win.destroy();
                        }
                    }
                }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
