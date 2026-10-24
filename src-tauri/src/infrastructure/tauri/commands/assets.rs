use crate::domain::services::link_resolution::{clean_asset_source, normalize_link_target};
use crate::infrastructure::tauri::state::AppState;
use std::path::{Path, PathBuf};
use tauri::State;

pub fn get_image_mime_type(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "tiff" | "tif" => "image/tiff",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub fn resolve_asset_file_path_internal(
    state: &AppState,
    src: &str,
    base_file: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(decoded) = clean_asset_source(src) else {
        return Ok(None);
    };

    #[allow(unused_mut)]
    let mut check_str = decoded.as_str();

    // En Unix, si venía de un esquema asset o ruta que perdió la barra inicial
    #[cfg(unix)]
    let linux_abs_buf: String;
    #[cfg(unix)]
    if !check_str.starts_with('/') && (check_str.starts_with("home/") || check_str.starts_with("tmp/") || check_str.starts_with("var/") || check_str.starts_with("usr/")) {
        linux_abs_buf = format!("/{}", check_str);
        check_str = &linux_abs_buf;
    }

    // 1. Si ya es una ruta absoluta en el sistema y es un archivo existente
    let path_buf = PathBuf::from(check_str);
    if path_buf.is_absolute() && path_buf.is_file() {
        return Ok(Some(path_buf.to_string_lossy().to_string()));
    }

    let vault_guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *vault_guard else {
        return Ok(None);
    };

    // 2. Intentar buscar en el DashMap (resolución O(1) e indexada de WikiLinks y subdirectorios con proximidad)
    let engine_guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    if let Some(ref engine) = *engine_guard {
        if let Some(resolved_rel) = engine.resolve_link_path_with_base(check_str, base_file) {
            let (rel_without_anchor, _) = normalize_link_target(&resolved_rel);
            let abs = vault_path.join(&rel_without_anchor);
            if abs.is_file() {
                return Ok(Some(abs.to_string_lossy().to_string()));
            }
        }
    }

    // 3. Intentar ruta relativa directa a la raíz de la bóveda
    let clean_rel = check_str.trim_start_matches("./").trim_start_matches('/');
    let direct_abs = vault_path.join(clean_rel);
    if direct_abs.is_file() {
        return Ok(Some(direct_abs.to_string_lossy().to_string()));
    }

    // 4. Intentar ruta relativa respecto al archivo actual (base_file)
    if let Some(base) = base_file {
        let base_clean = base.trim_start_matches("./").trim_start_matches('/');
        if let Some(parent) = Path::new(base_clean).parent() {
            let relative_abs = vault_path.join(parent).join(clean_rel);
            if relative_abs.is_file() {
                return Ok(Some(relative_abs.to_string_lossy().to_string()));
            }
        }
    }

    Ok(None)
}

#[tauri::command]
pub fn resolve_asset_file_path(
    state: State<'_, AppState>,
    src: String,
    base_file: Option<String>,
) -> Result<Option<String>, String> {
    match resolve_asset_file_path_internal(&state, &src, base_file.as_deref()) {
        Ok(Some(path)) => Ok(Some(path)),
        Ok(None) => {
            log::warn!("No se pudo resolver el asset: src={:?}, base_file={:?}", src, base_file);
            Ok(None)
        }
        Err(error) => {
            log::error!("Error resolviendo asset: src={:?}, base_file={:?}, error={}", src, base_file, error);
            Err(error)
        }
    }
}

#[tauri::command]
pub fn save_binary_asset(
    state: State<'_, AppState>,
    relative_path: String,
    base64_data: String,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;

    let raw_b64 = if let Some(idx) = base64_data.find(";base64,") {
        &base64_data[idx + 8..]
    } else {
        &base64_data
    };

    let bytes = BASE64_STANDARD
        .decode(raw_b64.trim())
        .map_err(|e| format!("Error decodificando base64: {}", e))?;

    state
        .file_system_use_cases
        .write_binary_item(vault_path, &relative_path, &bytes)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.reconcile_sync();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn read_asset_data_url(
    state: State<'_, AppState>,
    src: String,
    base_file: Option<String>,
) -> Result<Option<String>, String> {
    let resolved = match resolve_asset_file_path_internal(&state, &src, base_file.as_deref()) {
        Ok(resolved) => resolved,
        Err(error) => {
            log::error!("Error resolviendo asset para Data URL: src={:?}, base_file={:?}, error={}", src, base_file, error);
            return Err(error);
        }
    };
    let Some(abs_path) = resolved else {
        log::warn!("No se pudo leer asset como Data URL: src={:?}, base_file={:?}", src, base_file);
        return Ok(None);
    };

    let path = Path::new(&abs_path);
    if !path.is_file() {
        log::warn!("El asset resuelto no es un archivo: path={:?}", abs_path);
        return Ok(None);
    }

    let bytes = std::fs::read(path).map_err(|e| {
        log::error!("Error leyendo asset para Data URL: path={:?}, error={}", abs_path, e);
        format!("Error leyendo {}: {}", abs_path, e)
    })?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mime = get_image_mime_type(ext);

    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;
    let b64 = BASE64_STANDARD.encode(&bytes);
    Ok(Some(format!("data:{};base64,{}", mime, b64)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::file_types::SupportedFileTypes;
    use crate::infrastructure::navigation::engine::NavigationEngine;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_asset_file_path_and_data_url() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().to_path_buf();

        let sub_dir = vault_path.join("👷 Trabajo").join("adjuntos");
        std::fs::create_dir_all(&sub_dir).unwrap();
        let img_path = sub_dir.join("photo test.png");
        {
            let mut f = File::create(&img_path).unwrap();
            f.write_all(b"\x89PNG\r\n\x1a\nfakeimagebytes").unwrap();
        }

        let file_types = SupportedFileTypes::default();
        let repo = crate::infrastructure::repositories::file_note_repository::FileNoteRepository::new();
        let use_cases = crate::application::use_cases::note_use_cases::NoteUseCases::new(repo);
        let state = AppState::empty(file_types, use_cases);

        *state.active_vault_path.lock().unwrap() = Some(vault_path.clone());
        let engine = Arc::new(NavigationEngine::new(
            vault_path.clone(),
            vault_path.join(".synapse/cache.bin"),
        ));
        engine.reconcile_sync();
        *state.navigation_engine.lock().unwrap() = Some(engine);

        // 1. Direct absolute path
        let abs_res = resolve_asset_file_path_internal(&state, img_path.to_str().unwrap(), None).unwrap();
        assert_eq!(abs_res, Some(img_path.to_string_lossy().to_string()));

        // 2. Asset URL with encoded spaces, emoji and query param ?t=123
        let asset_url = format!("asset://localhost{}?t=123", img_path.to_string_lossy())
            .replace(' ', "%20")
            .replace("👷", "%F0%9F%91%B7");
        let asset_res = resolve_asset_file_path_internal(&state, &asset_url, None).unwrap();
        assert_eq!(asset_res, Some(img_path.to_string_lossy().to_string()));

        // 3. Vault relative path
        let rel_res = resolve_asset_file_path_internal(
            &state,
            "👷 Trabajo/adjuntos/photo test.png",
            None,
        ).unwrap();
        assert_eq!(rel_res, Some(img_path.to_string_lossy().to_string()));

        // 4. WikiLink lookup via DashMap: "photo test.png"
        let wiki_res = resolve_asset_file_path_internal(&state, "[[photo test.png]]", None).unwrap();
        assert_eq!(wiki_res, Some(img_path.to_string_lossy().to_string()));

        // 5. Embed WikiLink: "![[photo test.png]]"
        let embed_res = resolve_asset_file_path_internal(&state, "![[photo test.png]]", None).unwrap();
        assert_eq!(embed_res, Some(img_path.to_string_lossy().to_string()));

        // 6. Test reading Data URL directly from WikiLink
        let resolved = resolve_asset_file_path_internal(&state, "[[photo test.png]]", None).unwrap().unwrap();
        let bytes = std::fs::read(&resolved).unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(get_image_mime_type("png"), "image/png");
    }
}
