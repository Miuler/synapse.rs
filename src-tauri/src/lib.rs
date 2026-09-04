pub mod application;
pub mod domain;
pub mod infrastructure;

use application::use_cases::note_use_cases::NoteUseCases;
use domain::models::file_types::SupportedFileTypes;
use infrastructure::repositories::file_note_repository::FileNoteRepository;
use infrastructure::tauri::commands::{
    get_supported_file_types, get_vault_notes, read_note_content, save_note_content, search_items_command,
    search_notes_command, select_vault_folder, set_active_vault_path, AppState,
};
use std::env;
use std::path::PathBuf;
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
            read_note_content,
            save_note_content,
            set_active_vault_path,
            select_vault_folder,
            search_items_command,
            search_notes_command
        ])
        .setup(|app| {
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
