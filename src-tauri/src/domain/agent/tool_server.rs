use super::AgentLaunch;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Un servidor de herramientas (MCP) que se ofrece al agente. Puede alcanzarse por HTTP,
/// por un proceso local, o por ambos: el agente usa el que soporte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolServer {
    pub name: String,
    pub http_url: Option<String>,
    /// Cabeceras para la conexión HTTP; pueden llevar secretos.
    #[serde(default)]
    pub http_headers: BTreeMap<String, String>,
    pub process: Option<AgentLaunch>,
}
