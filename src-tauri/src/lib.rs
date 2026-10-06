pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod navigation;

use application::use_cases::note_use_cases::NoteUseCases;
use domain::models::file_types::SupportedFileTypes;
use infrastructure::repositories::file_note_repository::FileNoteRepository;
use infrastructure::tauri::commands::{
    delete_vault_item, full_text_search, get_active_vault_path, get_full_text_index_status,
    get_open_tabs_state, get_recent_notes_command, get_supported_file_types, get_system_theme,
    get_vault_directory_children, get_vault_files_count, get_vault_git_status, get_vault_notes,
    get_vault_ui_state, git_add_paths, git_restore_paths, git_restore_staged_paths,
    read_note_content, rebuild_full_text_index, record_note_opened, reload_vault_items, save_note_content,
    save_open_tabs_state, save_vault_ui_state, search_items_command, search_notes_command, select_vault_folder,
    set_active_vault_path, set_note_view_mode, toggle_devtools, AppState,
};
use std::env;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
//use webkit2gtk_nvidia_quirk::ApplyWorkaroundOptions;
#[cfg(target_os = "linux")]
use webkit2gtk_nvidia_quirk::{apply_workaround_with_options, ApplyWorkaroundOptions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Escaneamos por defecto la raíz del proyecto actual
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let default_vault = if cwd.ends_with("src-tauri") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        cwd
    };

    let file_types = SupportedFileTypes::default();
    let repo = FileNoteRepository::new();
    let use_cases = NoteUseCases::new(repo);
    let app_state = AppState::new(default_vault, file_types, use_cases);

    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        let options = ApplyWorkaroundOptions::default().force_disable_nv_explicit_sync(true);
        apply_workaround_with_options(options);
    }

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_supported_file_types,
            get_vault_notes,
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
            git_add_paths,
            git_restore_paths,
            git_restore_staged_paths,
            reload_vault_items,
            get_system_theme,
            full_text_search,
            get_full_text_index_status,
            rebuild_full_text_index
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            if let Ok(mut h) = state.app_handle.lock() {
                *h = Some(handle.clone());
            }
            if let Ok(engine) = state.navigation_engine.lock() {
                let h = handle.clone();
                engine.set_on_fs_change(std::sync::Arc::new(move |evt| {
                    use tauri::Emitter;
                    let _ = h.emit("vault:files-changed", &evt);
                }));
            }

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .targets([
                            Target::new(TargetKind::Stdout),
                            // Target::new(TargetKind::LogDir { fallback_to_logs: true }),
                            Target::new(TargetKind::Webview),
                        ])
                        .level(log::LevelFilter::Debug)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
