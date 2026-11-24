use crate::domain::agent::{
    AgentLaunch, AgentTranscript, ElicitationDecision, IterationTelemetry, PermissionDecision,
    PromptBlock, ToolServer,
};

pub struct ListAgentsQuery;

pub struct InstallAgentCommand {
    pub agent_id: String,
}

pub struct SaveAgentProfileCommand {
    pub id: String,
    pub name: String,
    pub launch: AgentLaunch,
}

pub struct DeleteAgentProfileCommand {
    pub id: String,
}

pub struct StartAgentSessionCommand {
    pub agent_id: String,
    pub cwd: String,
    /// Servidores de herramientas que se ofrecen al agente (el MCP de Synapse).
    pub tools: Vec<ToolServer>,
    /// Indicaciones para el agente al inicio de la conversación.
    pub instructions: Option<String>,
    /// Sesión ACP anterior que se intenta retomar, si el agente lo admite.
    pub resume: Option<String>,
}

pub struct SendAgentPromptCommand {
    pub session_id: String,
    pub blocks: Vec<PromptBlock>,
}

pub struct ChangeAgentModeCommand {
    pub session_id: String,
    pub mode_id: String,
}

pub struct ChangeAgentModelCommand {
    pub session_id: String,
    pub model_id: String,
}

pub struct CancelAgentPromptCommand {
    pub session_id: String,
}

pub struct RespondPermissionCommand {
    pub session_id: String,
    pub request_id: String,
    pub decision: PermissionDecision,
}

pub struct RespondElicitationCommand {
    pub session_id: String,
    pub request_id: String,
    pub decision: ElicitationDecision,
}

pub struct StopAgentSessionCommand {
    pub session_id: String,
}

pub struct SaveAgentTranscriptCommand {
    pub transcript: AgentTranscript,
}

pub struct ListAgentTranscriptsQuery {
    pub cwd: String,
}

pub struct LoadAgentTranscriptQuery {
    pub cwd: String,
    pub id: String,
}

pub struct DeleteAgentTranscriptCommand {
    pub cwd: String,
    pub id: String,
}

pub struct OpenAgentTranscriptCommand {
    pub cwd: String,
    pub id: String,
}

pub struct DeleteAllAgentTranscriptsCommand {
    pub cwd: String,
}

pub struct SetAgentProcessLimitCommand {
    pub agent_id: String,
    pub limit: usize,
}

pub struct RecordAgentIterationCommand {
    pub cwd: String,
    pub conversation_id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub acp_session_id: Option<String>,
    pub iteration: IterationTelemetry,
}
