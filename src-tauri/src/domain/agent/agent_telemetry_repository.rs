use super::{IterationTelemetry, McpCallTelemetry};

/// Persistencia de telemetría de agentes (sidecar por bóveda y registro global de uso).
pub trait AgentTelemetryRepository: Send + Sync {
    fn record_iteration(
        &self,
        cwd: &str,
        conversation_id: &str,
        agent_id: &str,
        agent_name: &str,
        acp_session_id: Option<&str>,
        iteration: IterationTelemetry,
    ) -> Result<(), String>;

    fn record_mcp_call(
        &self,
        cwd: &str,
        conversation_id: &str,
        call: McpCallTelemetry,
    ) -> Result<(), String>;
}
