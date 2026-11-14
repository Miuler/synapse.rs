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
