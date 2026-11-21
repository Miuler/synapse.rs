use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Resumen de una llamada a herramienta dentro de una iteración del agente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallTelemetry {
    pub tool_call_id: String,
    pub title: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub started_at: Option<i64>,
    #[serde(default)]
    pub ended_at: Option<i64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

/// Telemetría de una iteración (turno prompt -> respuesta) del agente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IterationTelemetry {
    pub id: String,
    pub turn_index: u32,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_ms: u64,
    pub stop_reason: String,
    #[serde(default)]
    pub usage_used: Option<u64>,
    #[serde(default)]
    pub usage_size: Option<u64>,
    #[serde(default)]
    pub cost_amount: Option<f64>,
    #[serde(default)]
    pub cost_delta: Option<f64>,
    #[serde(default)]
    pub cost_currency: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallTelemetry>,
    /// IDs de llamadas al servidor MCP de Synapse realizadas durante esta iteración.
    #[serde(default)]
    pub mcp_call_ids: Vec<String>,
}

/// Request y response de una llamada hecha por el agente al servidor MCP de Synapse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpCallTelemetry {
    pub id: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_ms: u64,
    pub method: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub jsonrpc_id: Option<Value>,
    pub request: Value,
    pub response: Value,
    #[serde(default)]
    pub is_error: bool,
}

/// Archivo sidecar por conversación en `<bóveda>/.synapse/agent-sessions/telemetry/<id>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSessionTelemetry {
    pub conversation_id: String,
    pub transcript_path: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub agent_name: String,
    pub cwd: String,
    #[serde(default)]
    pub acp_session_id: Option<String>,
    #[serde(default)]
    pub iterations: Vec<IterationTelemetry>,
    #[serde(default)]
    pub mcp_calls: Vec<McpCallTelemetry>,
}

/// Registro global de uso por iteración en `~/.config/synapsers/agent-usage.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentUsageRecord {
    pub id: String,
    pub recorded_at: i64,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_ms: u64,
    pub agent_id: String,
    pub agent_name: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub vault_cwd: String,
    pub conversation_id: String,
    #[serde(default)]
    pub acp_session_id: Option<String>,
    /// Ruta al JSON de la conversación en la bóveda que generó este registro.
    pub transcript_path: String,
    /// Ruta al JSON de telemetría detallada de esta conversación en la bóveda.
    pub telemetry_path: String,
    pub turn_index: u32,
    pub stop_reason: String,
    #[serde(default)]
    pub usage_used: Option<u64>,
    #[serde(default)]
    pub usage_size: Option<u64>,
    #[serde(default)]
    pub cost_amount: Option<f64>,
    #[serde(default)]
    pub cost_delta: Option<f64>,
    #[serde(default)]
    pub cost_currency: Option<String>,
    pub tool_calls_count: u32,
    pub mcp_calls_count: u32,
}
