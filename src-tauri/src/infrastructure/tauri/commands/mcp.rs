use crate::domain::models::mcp::{McpPortRegistry, McpRuntimeStatus};
use crate::infrastructure::mcp::port_registry;
use crate::infrastructure::tauri::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_mcp_server_status(state: State<'_, AppState>) -> Result<McpRuntimeStatus, String> {
    Ok(state.mcp_runtime.get_status())
}

#[tauri::command]
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
    state.mcp_runtime.stop()
}
