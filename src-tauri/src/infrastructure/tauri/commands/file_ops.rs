use crate::infrastructure::tauri::state::AppState;
use tauri::State;

#[tauri::command]
pub fn delete_vault_item(state: State<'_, AppState>, relative_path: String) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state.file_system_use_cases.delete_item(vault_path, &relative_path)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.spawn_reconciliation();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn rename_vault_item(
    state: State<'_, AppState>,
    relative_path: String,
    new_name: String,
) -> Result<String, String> {
    let vault_path = {
        let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        match *guard {
            Some(ref path) => path.clone(),
            None => return Err("No hay ninguna bóveda abierta".to_string()),
        }
    };

    // Si estaba versionado, el renombrado se deja preparado en el índice (git detecta el rename)
    let was_tracked = crate::infrastructure::services::git_service::GitService::new()
        .is_tracked(&vault_path, &relative_path);

    let renamed_path = state.file_system_use_cases.rename_item(&vault_path, &relative_path, &new_name)?;
    if was_tracked && renamed_path != relative_path {
        if let Err(err) = state
            .git_use_cases
            .add_paths(&vault_path, &[relative_path.clone(), renamed_path.clone()])
        {
            log::warn!("[rename_vault_item] No se pudo preparar el renombrado en git: {}", err);
        }
    }
    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.spawn_reconciliation();
        }
    }
    Ok(renamed_path)
}

/// Elemento movido: ruta relativa de origen y de destino.
#[derive(serde::Serialize)]
pub struct MovedItem {
    pub from: String,
    pub to: String,
}

/// Mueve los elementos indicados a `dest_dir`. Si estaban versionados, deja el movimiento
/// preparado en el índice de Git (rename). Retorna los pares origen/destino realmente movidos.
#[tauri::command]
pub fn move_vault_items(
    state: State<'_, AppState>,
    paths: Vec<String>,
    dest_dir: String,
) -> Result<Vec<MovedItem>, String> {
    let vault_path = {
        let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        match *guard {
            Some(ref p) => p.clone(),
            None => return Err("No hay ninguna bóveda abierta".to_string()),
        }
    };

    let git = crate::infrastructure::services::git_service::GitService::new();
    let mut moved = Vec::with_capacity(paths.len());
    let mut first_error: Option<String> = None;
    for source in &paths {
        let was_tracked = git.is_tracked(&vault_path, source);
        match state.file_system_use_cases.move_item(&vault_path, source, &dest_dir) {
            Ok(to) if &to != source => {
                if was_tracked {
                    if let Err(err) = state.git_use_cases.add_paths(&vault_path, &[source.clone(), to.clone()]) {
                        log::warn!("[move_vault_items] No se pudo preparar el movimiento en git: {}", err);
                    }
                }
                moved.push(MovedItem { from: source.clone(), to });
            }
            Ok(_) => {}
            Err(err) => {
                first_error.get_or_insert(err);
            }
        }
    }

    if !moved.is_empty() {
        if let Ok(guard) = state.navigation_engine.lock() {
            if let Some(ref engine) = *guard {
                engine.spawn_reconciliation();
            }
        }
    }
    // Si algo se movió se devuelve lo hecho; el error solo se propaga cuando no se movió nada
    match (moved.is_empty(), first_error) {
        (true, Some(err)) => Err(err),
        _ => Ok(moved),
    }
}

/// Pega (copia) los elementos indicados dentro de `dest_dir`.
/// Retorna las rutas relativas de los nuevos elementos creados.
#[tauri::command]
pub fn copy_vault_items(
    state: State<'_, AppState>,
    paths: Vec<String>,
    dest_dir: String,
) -> Result<Vec<String>, String> {
    let vault_path = {
        let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
        match *guard {
            Some(ref p) => p.clone(),
            None => return Err("No hay ninguna bóveda abierta".to_string()),
        }
    };

    let created = state.file_system_use_cases.copy_items(&vault_path, &paths, &dest_dir)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            let _ = engine.reload_paths(&created);
        }
    }

    Ok(created)
}

#[tauri::command]
pub fn reload_vault_items(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<String>, String> {
    let guard = state
        .navigation_engine
        .lock()
        .map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(Vec::new());
    };
    engine.reload_paths(&paths)
}

#[tauri::command]
pub fn read_binary_file(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<Vec<u8>, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state.file_system_use_cases.read_binary_item(vault_path, &relative_path)
}

#[tauri::command]
pub fn write_binary_file(
    state: State<'_, AppState>,
    relative_path: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state
        .file_system_use_cases
        .write_binary_item(vault_path, &relative_path, &data)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.spawn_reconciliation();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn create_vault_folder(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state
        .file_system_use_cases
        .create_folder(vault_path, &relative_path)?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.spawn_reconciliation();
        }
    }

    Ok(())
}
