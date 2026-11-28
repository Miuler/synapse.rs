//! Actualizaciones de la app: comprobar `latest.json` de la última release, descargar el paquete
//! (el plugin verifica la firma con la clave pública de `tauri.conf.json`) e instalarlo.

#[cfg(feature = "desktop")]
use crate::infrastructure::service::os::self_update;
use serde::Serialize;
#[cfg(feature = "desktop")]
use tauri::Emitter;
use tauri::AppHandle;
#[cfg(feature = "desktop")]
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    /// Novedades de la versión (Markdown del CHANGELOG)
    pub notes: Option<String>,
    /// Las mismas en español (`notes_es` de latest.json)
    pub notes_es: Option<String>,
    pub date: Option<String>,
}

#[cfg(feature = "desktop")]
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
}

#[cfg(feature = "desktop")]
async fn find_update(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    app.updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("Could not check latest version: {e}"))
}

/// Devuelve la versión nueva si la hay.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    #[cfg(feature = "desktop")]
    {
        Ok(find_update(&app).await?.map(|u| UpdateInfo {
            version: u.version.clone(),
            current_version: u.current_version.clone(),
            notes: u.body.clone(),
            notes_es: u.raw_json.get("notes_es").and_then(|n| n.as_str()).map(String::from),
            date: u.date.map(|d| d.to_string()),
        }))
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = app;
        Ok(None)
    }
}

/// Descarga e instala la versión nueva. Emite `app-update-progress` mientras descarga.
/// Queda instalada para el próximo arranque (`restart_app` la aplica ya).
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<String, String> {
    #[cfg(feature = "desktop")]
    {
        let update = find_update(&app).await?.ok_or("already on the latest version")?;
        let mut downloaded: u64 = 0;
        let emitter = app.clone();
        let bytes = update
            .download(
                move |chunk, total| {
                    downloaded += chunk as u64;
                    let _ = emitter.emit("app-update-progress", UpdateProgress { downloaded, total });
                },
                || {},
            )
            .await
            .map_err(|e| format!("Could not download or verify update: {e}"))?;

        if cfg!(target_os = "macos") {
            // .app.tar.gz: el instalador del plugin sí sabe reemplazar el bundle
            update.install(bytes).map_err(|e| format!("Could not install update: {e}"))?;
        } else {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let exe_name = exe.file_name().and_then(|n| n.to_str()).ok_or("invalid executable name")?;
            let binary = self_update::extract_binary(&bytes, exe_name)?;
            self_update::replace_executable(&exe, &binary)?;
        }
        Ok(update.version)
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = app;
        Err("automatic updates are only available on desktop".to_string())
    }
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}
