pub mod application;
pub mod domain;
pub mod infrastructure;

use infrastructure::tauri::commands::{
    get_vault_notes, read_note_content, save_note_content, set_active_vault_path, AppState,
};
use std::env;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Definimos una ruta por defecto para el Vault (ejemplo: ~/SynapseVault)
    let default_vault = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let app_state = AppState::new(default_vault);

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_vault_notes,
            read_note_content,
            save_note_content,
            set_active_vault_path
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
