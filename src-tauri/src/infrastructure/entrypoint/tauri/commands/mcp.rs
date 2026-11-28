use crate::application::dto::issue::source_draft::SourceDraft;
use crate::domain::mcp::mcp::McpPortRegistry;
use crate::infrastructure::config::state::AppState;
use crate::infrastructure::entrypoint::mcp::dto::McpRuntimeStatus;
use crate::infrastructure::entrypoint::mcp::port_registry;
use tauri::State;

#[tauri::command(async)]
pub fn get_mcp_server_status(state: State<'_, AppState>) -> Result<McpRuntimeStatus, String> {
    Ok(state.mcp_runtime.get_status())
}

#[tauri::command(async)]
pub fn get_mcp_port_registry() -> Result<McpPortRegistry, String> {
    Ok(port_registry::read_global_registry())
}

#[tauri::command]
pub fn toggle_mcp_server(state: State<'_, AppState>) -> Result<McpRuntimeStatus, String> {
    state.mcp_runtime.toggle()
}

#[tauri::command]
pub fn start_mcp_server(state: State<'_, AppState>) -> Result<McpRuntimeStatus, String> {
    state.mcp_runtime.start()
}

#[tauri::command]
pub fn stop_mcp_server(state: State<'_, AppState>) -> Result<McpRuntimeStatus, String> {
    state.mcp_runtime.disable()
}

/// Respuesta de la ventana a `mcp://render-diagram`: la imagen en base64 o el error.
#[tauri::command]
pub fn mcp_diagram_rendered(
    state: State<'_, AppState>,
    request_id: String,
    data: Option<String>,
    mime_type: Option<String>,
    error: Option<String>,
) {
    use crate::application::service::diagram_renderer::RenderedImage;
    let result = match (data, error) {
        (Some(data_base64), None) => Ok(RenderedImage {
            mime_type: mime_type.unwrap_or_else(|| "image/png".into()),
            data_base64,
        }),
        (_, err) => Err(err.unwrap_or_else(|| "La ventana no devolvió la imagen".into())),
    };
    state.diagram_renderer.resolve(&request_id, result);
}

/// La configuración que el usuario tiene abierta, cada vez que cambia; `null` al cerrarla.
#[tauri::command]
pub fn mcp_config_draft(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    draft: Option<SourceDraft>,
) {
    state.config_window.set(window.label(), draft);
}

/// Respuesta de la ventana a `mcp://config-change`: el borrador como quedó, o el motivo.
#[tauri::command]
pub fn mcp_config_changed(
    state: State<'_, AppState>,
    request_id: String,
    draft: Option<SourceDraft>,
    error: Option<String>,
) {
    let reply = match (draft, error) {
        (Some(draft), None) => Ok(draft),
        (_, error) => Err(error.unwrap_or_else(|| "La ventana no aplicó el cambio".into())),
    };
    state.config_window.changed(&request_id, reply);
}

/// Respuesta de la ventana a `mcp://conversations`.
#[tauri::command]
pub fn mcp_conversations_reply(
    state: State<'_, AppState>,
    request_id: String,
    result: Option<serde_json::Value>,
    error: Option<String>,
) {
    let outcome = match (result, error) {
        (_, Some(e)) => Err(e),
        (Some(v), None) => Ok(v),
        (None, None) => Err("La ventana no respondió".into()),
    };
    state.conversation_hub.resolve(&request_id, outcome);
}

/// Respuesta del usuario a `mcp://confirm`.
#[tauri::command]
pub fn mcp_confirm_reply(state: State<'_, AppState>, request_id: String, allowed: bool) {
    state.user_prompter.resolve(&request_id, allowed);
}

/// Conversaciones de agentes de las otras instancias de Synapse abiertas.
#[tauri::command(async)]
pub fn peer_list(request: serde_json::Value) -> Vec<serde_json::Value> {
    use crate::application::service::peer_instances::PeerInstances;
    crate::infrastructure::integration::service::peer_instances_impl::PeerInstancesImpl
        .broadcast(request)
}

/// Pedido a la instancia de otra bóveda (entregar un mensaje o una respuesta).
#[tauri::command(async)]
pub fn peer_send(vault: String, request: serde_json::Value) -> Result<serde_json::Value, String> {
    use crate::application::service::peer_instances::PeerInstances;
    crate::infrastructure::integration::service::peer_instances_impl::PeerInstancesImpl
        .send(&vault, request)
}

/// Nombre y ruta de la bóveda de esta instancia (para presentarse ante las demás).
#[tauri::command]
pub fn peer_identity(state: State<'_, AppState>) -> Option<serde_json::Value> {
    let path = state.active_vault_path.lock().ok()?.clone()?;
    let path = path.canonicalize().unwrap_or(path);
    Some(serde_json::json!({
        "vault": path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        "path": path.to_string_lossy(),
    }))
}
