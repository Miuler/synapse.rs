use crate::domain::models::startup::StartupTarget;
use crate::infrastructure::tauri::state::AppState;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn toggle_devtools(window: tauri::WebviewWindow) {
    if window.is_devtools_open() {
        window.close_devtools();
    } else {
        window.open_devtools();
    }
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
pub fn show_item_in_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state.file_system_use_cases.show_item_in_folder(vault_path, &path)
}

#[tauri::command]
pub async fn get_system_theme(window: tauri::WebviewWindow) -> String {
    #[cfg(target_os = "linux")]
    {
        let detected = tauri::async_runtime::spawn_blocking(|| -> Option<String> {
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
                        return Some("dark".to_string());
                    } else if stdout.contains("u 2") {
                        return Some("light".to_string());
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
                        return Some("dark".to_string());
                    } else if stdout.contains("light") {
                        return Some("light".to_string());
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
                        return Some("dark".to_string());
                    } else if theme.contains("light") || theme.contains("white") {
                        return Some("light".to_string());
                    }
                }
            }

            None
        })
        .await
        .ok()
        .flatten();

        if let Some(theme) = detected {
            return theme;
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

#[cfg(test)]
mod tests {
    use super::*;

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
