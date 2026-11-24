//! Agentes de IA externos con los que el usuario conversa desde Synapse.

pub mod agent_profile;
pub mod agent_quota;
pub mod agent_profile_repository;
pub mod agent_session;
pub mod agent_telemetry;
pub mod agent_telemetry_repository;
pub mod agent_transcript;
pub mod agent_transcript_repository;
pub mod elicitation;
pub mod permission_request;
pub mod prompt_block;
pub mod tool_server;

pub use agent_profile::{AgentLaunch, AgentProfile, AgentSource};
pub use agent_profile_repository::AgentProfileRepository;
pub use agent_quota::{AgentQuota, QuotaGroup, QuotaWindow};
pub use agent_session::{AgentCapabilities, AgentConfig, AgentOption, AgentSession};
pub use agent_telemetry::{
    AgentSessionTelemetry, AgentUsageRecord, IterationTelemetry, McpCallTelemetry,
    ToolCallTelemetry,
};
pub use agent_telemetry_repository::AgentTelemetryRepository;
pub use agent_transcript::{AgentTranscript, AgentTranscriptSummary};
pub use agent_transcript_repository::AgentTranscriptRepository;
pub use elicitation::ElicitationDecision;
pub use permission_request::{PermissionDecision, PermissionOption, PermissionRequest};
pub use prompt_block::PromptBlock;
pub use tool_server::ToolServer;
