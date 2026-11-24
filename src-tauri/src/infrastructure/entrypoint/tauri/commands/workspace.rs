use crate::domain::config::config::ShortcutsConfig;
use crate::domain::workspace::workspace_registry::RecentWorkspaceEntry;
use crate::infrastructure::config::state::AppState;
use crate::infrastructure::entrypoint::tauri::dto::SelectVaultFolderResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultDirectoryList {
    pub files: Vec<String>,
    pub folders: Vec<String>,
}

#[tauri::command]
pub async fn select_vault_folder(
    state: State<'_, AppState>,
    starting_directory: Option<String>,
) -> Result<Option<SelectVaultFolderResult>, String> {
    #[cfg(feature = "desktop")]
    {
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

            let res = state.activate_vault(path, None)?;
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = (state, starting_directory);
        Ok(None)
    }
}

#[tauri::command]
pub async fn get_recent_workspaces(
    state: State<'_, AppState>,
) -> Result<Vec<RecentWorkspaceEntry>, String> {
    state
        .list_workspaces_use_case
        .execute(crate::application::dto::workspace::list_workspaces_query::ListWorkspacesQuery {})
}

#[tauri::command]
pub async fn get_shortcuts_config(state: State<'_, AppState>) -> Result<ShortcutsConfig, String> {
    state.get_shortcuts_config_use_case.execute(
        crate::application::dto::workspace::get_shortcuts_config_query::GetShortcutsConfigQuery {},
    )
}

#[tauri::command]
pub async fn save_shortcut(
    state: State<'_, AppState>,
    section: String,
    action: String,
    value: String,
) -> Result<ShortcutsConfig, String> {
    state.save_shortcut_use_case.execute(
        crate::application::dto::workspace::save_shortcut_command::SaveShortcutCommand {
            section,
            action,
            value,
        },
    )
}

/// Icono de las notificaciones del sistema (el pulpo kawaii). El demonio de notificaciones lee un
/// archivo real, no un recurso embebido en el ejecutable: se vuelca a la caché de la app al usarlo.
const NOTIFICATION_ICON: &[u8] = include_bytes!("../../../../../icons/notification-pulpo.png");
const NOTIFICATION_ICON_FILE: &str = "notification-pulpo.png";

/// Escribe el icono en `dir` si falta o cambió (otra versión de la app) y devuelve su ruta.
fn materialize_notification_icon(dir: &std::path::Path) -> Option<PathBuf> {
    let path = dir.join(NOTIFICATION_ICON_FILE);
    let up_to_date = std::fs::read(&path).is_ok_and(|bytes| bytes == NOTIFICATION_ICON);
    if !up_to_date {
        std::fs::create_dir_all(dir).ok()?;
        std::fs::write(&path, NOTIFICATION_ICON).ok()?;
    }
    Some(path)
}

/// En Linux el cuerpo admite marcado: un `<` o `&` de la consulta del usuario lo rompería.
fn escape_notification_body(body: String) -> String {
    if cfg!(target_os = "linux") {
        body.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    } else {
        body
    }
}

/// Notificación del sistema con el icono de Synapse (sin él, el demonio muestra uno genérico).
fn show_notification(window: &tauri::WebviewWindow, title: String, body: String) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_notification::NotificationExt;
    let mut builder = window.notification().builder().title(title).body(escape_notification_body(body));
    let icon = window
        .app_handle()
        .path()
        .app_cache_dir()
        .ok()
        .and_then(|dir| materialize_notification_icon(&dir));
    if let Some(icon) = icon {
        builder = builder.icon(icon.to_string_lossy().into_owned());
    }
    builder.show().map_err(|e| e.to_string())
}

/// Pide la atención del usuario cuando Synapse espera una respuesta (un permiso, una pregunta) y la
/// ventana no está al frente: la marca como urgente en la barra de tareas y avisa con una
/// notificación del sistema. No le quita el foco a lo que el usuario esté haciendo.
#[tauri::command]
pub fn request_attention(window: tauri::WebviewWindow, title: String, body: String) -> Result<(), String> {
    if window.is_focused().unwrap_or(false) {
        return Ok(());
    }
    let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
    show_notification(&window, title, body)
}

/// Avisa con una notificación del sistema solo si ninguna ventana de este proceso tiene el foco (por
/// ejemplo, cuando el agente termina y el usuario está en otra aplicación). Sin marca de urgencia.
/// Otra instancia de Synapse con otra bóveda es otro proceso y no cuenta.
#[tauri::command]
pub fn notify_if_unfocused(window: tauri::WebviewWindow, title: String, body: String) -> Result<(), String> {
    use tauri::Manager;
    let focused = window
        .app_handle()
        .webview_windows()
        .values()
        .any(|w| w.is_focused().unwrap_or(false));
    if focused {
        return Ok(());
    }
    show_notification(&window, title, body)
}

/// Diálogo nativo para elegir una imagen; devuelve su ruta absoluta.
#[tauri::command]
pub async fn pick_image_file() -> Result<Option<String>, String> {
    #[cfg(feature = "desktop")]
    {
        let file = rfd::AsyncFileDialog::new()
            .set_title("Seleccionar imagen de fondo")
            .add_filter(
                "Imágenes",
                &["png", "jpg", "jpeg", "webp", "gif", "avif", "bmp", "svg"],
            )
            .pick_file()
            .await;
        Ok(file.map(|f| f.path().to_string_lossy().into_owned()))
    }
    #[cfg(not(feature = "desktop"))]
    {
        Ok(None)
    }
}

#[tauri::command]
pub async fn toggle_pin_workspace(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<RecentWorkspaceEntry>, String> {
    state
        .toggle_workspace_pin_use_case
        .execute(crate::application::dto::workspace::toggle_workspace_pin_command::ToggleWorkspacePinCommand { path })
}

#[tauri::command]
pub async fn reorder_pinned_workspaces(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<RecentWorkspaceEntry>, String> {
    state.reorder_pinned_workspaces_use_case.execute(
        crate::application::dto::workspace::reorder_pinned_workspaces_command::ReorderPinnedWorkspacesCommand {
            paths,
        },
    )
}

#[tauri::command]
pub async fn remove_recent_workspace(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<RecentWorkspaceEntry>, String> {
    state.remove_workspace_use_case.execute(
        crate::application::dto::workspace::remove_workspace_command::RemoveWorkspaceCommand {
            path,
        },
    )
}

#[tauri::command]
pub async fn create_new_workspace(
    state: State<'_, AppState>,
    starting_directory: Option<String>,
) -> Result<Option<SelectVaultFolderResult>, String> {
    #[cfg(feature = "desktop")]
    {
        let mut dialog = rfd::AsyncFileDialog::new()
            .set_title("Seleccionar o Crear Directorio para Nuevo Workspace");

        if let Some(dir) = starting_directory
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
        {
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

            let synapse_dir = path.join(".synapse");
            if let Err(e) = std::fs::create_dir_all(&synapse_dir) {
                return Err(format!("No se pudo crear el directorio .synapse: {}", e));
            }

            let res = state.activate_vault(path, None)?;
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = (state, starting_directory);
        Ok(None)
    }
}

#[tauri::command]
pub async fn clone_git_workspace(
    state: State<'_, AppState>,
    repo_url: String,
    starting_directory: Option<String>,
) -> Result<Option<SelectVaultFolderResult>, String> {
    #[cfg(feature = "desktop")]
    {
        let trimmed_url = repo_url.trim().to_string();
        if trimmed_url.is_empty() {
            return Err("La URL del repositorio Git no puede estar vacía".to_string());
        }

        let mut dialog = rfd::AsyncFileDialog::new()
            .set_title("Seleccionar Carpeta Destino para Clonar Repositorio");

        if let Some(dir) = starting_directory
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
        {
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

        state.clone_repository_use_case.execute(
            crate::application::dto::version::clone_repository_command::CloneRepositoryCommand {
                repo_url: trimmed_url.to_string(),
                target_path: target_dir.clone(),
            },
        )?;

        let synapse_dir = target_dir.join(".synapse");
        let _ = std::fs::create_dir_all(&synapse_dir);

        let res = state.activate_vault(target_dir, Some(true))?;
        Ok(Some(res))
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = (state, repo_url, starting_directory);
        Ok(None)
    }
}

#[tauri::command]
pub fn open_workspace_in_new_window(app: tauri::AppHandle, path: String) -> Result<(), String> {
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

#[tauri::command(async)]
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

#[tauri::command(async)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_materialize_notification_icon_writes_and_refreshes() {
        let dir = std::env::temp_dir().join("synapse_notification_icon_test");
        let _ = std::fs::remove_dir_all(&dir);

        let path = materialize_notification_icon(&dir).expect("debe volcar el icono");
        assert_eq!(std::fs::read(&path).unwrap(), NOTIFICATION_ICON);

        // El de otra versión se reemplaza, aunque tenga el mismo tamaño
        std::fs::write(&path, vec![0u8; NOTIFICATION_ICON.len()]).unwrap();
        materialize_notification_icon(&dir).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), NOTIFICATION_ICON);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
