use super::AgentLaunch;
use serde::{Deserialize, Serialize};

/// Un servidor de herramientas (MCP) que se ofrece al agente. Puede alcanzarse por HTTP,
/// por un proceso local, o por ambos: el agente usa el que soporte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolServer {
    pub name: String,
    pub http_url: Option<String>,
    pub process: Option<AgentLaunch>,
}
