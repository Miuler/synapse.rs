use crate::application::dto::vault_item::copy_vault_items_command::CopyVaultItemsCommand;
use crate::application::dto::vault_item::create_folder_command::CreateFolderCommand;
use crate::application::dto::vault_item::delete_vault_item_command::DeleteVaultItemCommand;
use crate::application::dto::vault_item::move_vault_items_command::MoveVaultItemsCommand;
use crate::application::dto::vault_item::read_binary_vault_item_query::ReadBinaryVaultItemQuery;
use crate::application::dto::vault_item::rename_vault_item_command::RenameVaultItemCommand;
use crate::application::dto::vault_item::write_binary_vault_item_command::WriteBinaryVaultItemCommand;
use crate::infrastructure::config::state::AppState;
use tauri::State;

#[tauri::command]
pub fn delete_vault_item(state: State<'_, AppState>, relative_path: String) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };

    state.delete_vault_item_use_case.execute(DeleteVaultItemCommand {
        vault_path: vault_path.clone(),
        relative_path,
    })?;

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

    let renamed_path = state.rename_vault_item_use_case.execute(RenameVaultItemCommand {
        vault_path,
        relative_path,
        new_name,
    })?;
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

    let result = state.move_vault_items_use_case.execute(MoveVaultItemsCommand {
        vault_path,
        paths,
        dest_dir,
    });
    let moved: Vec<MovedItem> = match result {
        Ok(items) => items.into_iter().map(|m| MovedItem { from: m.from, to: m.to }).collect(),
        Err(err) => return Err(err),
    };

    if !moved.is_empty() {
        if let Ok(guard) = state.navigation_engine.lock() {
            if let Some(ref engine) = *guard {
                engine.spawn_reconciliation();
            }
        }
    }
    Ok(moved)
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

    let created = state.copy_vault_items_use_case.execute(CopyVaultItemsCommand {
        vault_path,
        paths,
        dest_dir,
    })?;

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

    state.read_binary_vault_item_use_case.execute(ReadBinaryVaultItemQuery {
        vault_path: vault_path.clone(),
        relative_path,
    })
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

    state.write_binary_vault_item_use_case.execute(WriteBinaryVaultItemCommand {
        vault_path: vault_path.clone(),
        relative_path,
        bytes: data,
    })?;

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

    state.create_folder_use_case.execute(CreateFolderCommand {
        vault_path: vault_path.clone(),
        relative_path,
    })?;

    if let Ok(guard) = state.navigation_engine.lock() {
        if let Some(ref engine) = *guard {
            engine.spawn_reconciliation();
        }
    }

    Ok(())
}
