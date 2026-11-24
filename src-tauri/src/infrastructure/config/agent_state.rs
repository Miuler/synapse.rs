//! Ensamblado de los casos de uso de agentes con sus implementaciones.

use crate::application::use_case::cancel_agent_prompt_use_case::CancelAgentPromptUseCase;
use crate::application::use_case::change_agent_mode_use_case::ChangeAgentModeUseCase;
use crate::application::use_case::change_agent_model_use_case::ChangeAgentModelUseCase;
use crate::application::use_case::change_agent_option_use_case::ChangeAgentOptionUseCase;
use crate::application::use_case::delete_agent_profile_use_case::DeleteAgentProfileUseCase;
use crate::application::use_case::delete_agent_transcript_use_case::DeleteAgentTranscriptUseCase;
use crate::application::use_case::delete_all_agent_transcripts_use_case::DeleteAllAgentTranscriptsUseCase;
use crate::application::use_case::install_agent_use_case::InstallAgentUseCase;
use crate::application::use_case::list_agent_transcripts_use_case::ListAgentTranscriptsUseCase;
use crate::application::use_case::list_agents_use_case::ListAgentsUseCase;
use crate::application::use_case::load_agent_transcript_use_case::LoadAgentTranscriptUseCase;
use crate::application::use_case::open_agent_transcript_use_case::OpenAgentTranscriptUseCase;
use crate::application::use_case::record_agent_iteration_use_case::RecordAgentIterationUseCase;
use crate::application::use_case::respond_elicitation_use_case::RespondElicitationUseCase;
use crate::application::use_case::respond_permission_use_case::RespondPermissionUseCase;
use crate::application::use_case::save_agent_profile_use_case::SaveAgentProfileUseCase;
use crate::application::use_case::save_agent_transcript_use_case::SaveAgentTranscriptUseCase;
use crate::application::use_case::send_agent_prompt_use_case::SendAgentPromptUseCase;
use crate::application::use_case::set_agent_process_limit_use_case::SetAgentProcessLimitUseCase;
use crate::application::use_case::start_agent_session_use_case::StartAgentSessionUseCase;
use crate::application::use_case::stop_agent_session_use_case::StopAgentSessionUseCase;
use crate::infrastructure::integration::service::acp::registry::RegistryClient;
use crate::infrastructure::integration::service::acp::{
    AgentCatalogImpl, AgentEventSink, AgentInstallerImpl, AgentRunnerImpl,
};
use crate::infrastructure::repository::file::agent_profile_repository_impl::AgentProfileRepositoryImpl;
use crate::infrastructure::repository::file::agent_telemetry_repository_impl::AgentTelemetryRepositoryImpl;
use crate::infrastructure::repository::file::agent_transcript_repository_impl::AgentTranscriptRepositoryImpl;
use crate::infrastructure::service::os::system_shell_impl::SystemShellImpl;

pub struct AgentState {
    pub list_agents: ListAgentsUseCase<AgentCatalogImpl, AgentProfileRepositoryImpl>,
    pub install_agent: InstallAgentUseCase<AgentInstallerImpl>,
    pub save_agent_profile: SaveAgentProfileUseCase<AgentProfileRepositoryImpl>,
    pub delete_agent_profile: DeleteAgentProfileUseCase<AgentProfileRepositoryImpl>,
    pub start_agent_session:
        StartAgentSessionUseCase<AgentProfileRepositoryImpl, AgentInstallerImpl, AgentRunnerImpl>,
    pub send_agent_prompt: SendAgentPromptUseCase<AgentRunnerImpl>,
    pub cancel_agent_prompt: CancelAgentPromptUseCase<AgentRunnerImpl>,
    pub respond_permission: RespondPermissionUseCase<AgentRunnerImpl>,
    pub respond_elicitation: RespondElicitationUseCase<AgentRunnerImpl>,
    pub change_agent_mode: ChangeAgentModeUseCase<AgentRunnerImpl>,
    pub change_agent_model: ChangeAgentModelUseCase<AgentRunnerImpl>,
    pub change_agent_option: ChangeAgentOptionUseCase<AgentRunnerImpl>,
    pub stop_agent_session: StopAgentSessionUseCase<AgentRunnerImpl>,
    pub set_agent_process_limit: SetAgentProcessLimitUseCase<AgentRunnerImpl>,
    pub save_agent_transcript: SaveAgentTranscriptUseCase<AgentTranscriptRepositoryImpl>,
    pub list_agent_transcripts: ListAgentTranscriptsUseCase<AgentTranscriptRepositoryImpl>,
    pub load_agent_transcript: LoadAgentTranscriptUseCase<AgentTranscriptRepositoryImpl>,
    pub delete_agent_transcript: DeleteAgentTranscriptUseCase<AgentTranscriptRepositoryImpl>,
    pub delete_all_agent_transcripts:
        DeleteAllAgentTranscriptsUseCase<AgentTranscriptRepositoryImpl>,
    pub open_agent_transcript:
        OpenAgentTranscriptUseCase<AgentTranscriptRepositoryImpl, SystemShellImpl>,
    pub record_agent_iteration: RecordAgentIterationUseCase<AgentTelemetryRepositoryImpl>,
}

impl AgentState {
    pub fn new(sink: AgentEventSink) -> Self {
        let registry = RegistryClient::new();
        let catalog = AgentCatalogImpl::new(registry.clone());
        let installer = AgentInstallerImpl::new(registry);
        let profiles = AgentProfileRepositoryImpl::default();
        let runner = AgentRunnerImpl::new(sink);
        let transcripts = AgentTranscriptRepositoryImpl;
        let telemetry = AgentTelemetryRepositoryImpl::default();
        Self {
            list_agents: ListAgentsUseCase::new(catalog, profiles.clone()),
            install_agent: InstallAgentUseCase::new(installer.clone()),
            save_agent_profile: SaveAgentProfileUseCase::new(profiles.clone()),
            delete_agent_profile: DeleteAgentProfileUseCase::new(profiles.clone()),
            start_agent_session: StartAgentSessionUseCase::new(profiles, installer, runner.clone()),
            send_agent_prompt: SendAgentPromptUseCase::new(runner.clone()),
            cancel_agent_prompt: CancelAgentPromptUseCase::new(runner.clone()),
            respond_permission: RespondPermissionUseCase::new(runner.clone()),
            respond_elicitation: RespondElicitationUseCase::new(runner.clone()),
            change_agent_mode: ChangeAgentModeUseCase::new(runner.clone()),
            change_agent_model: ChangeAgentModelUseCase::new(runner.clone()),
            change_agent_option: ChangeAgentOptionUseCase::new(runner.clone()),
            set_agent_process_limit: SetAgentProcessLimitUseCase::new(runner.clone()),
            stop_agent_session: StopAgentSessionUseCase::new(runner),
            save_agent_transcript: SaveAgentTranscriptUseCase::new(transcripts.clone()),
            list_agent_transcripts: ListAgentTranscriptsUseCase::new(transcripts.clone()),
            load_agent_transcript: LoadAgentTranscriptUseCase::new(transcripts.clone()),
            delete_agent_transcript: DeleteAgentTranscriptUseCase::new(transcripts.clone()),
            delete_all_agent_transcripts: DeleteAllAgentTranscriptsUseCase::new(
                transcripts.clone(),
            ),
            open_agent_transcript: OpenAgentTranscriptUseCase::new(transcripts, SystemShellImpl),
            record_agent_iteration: RecordAgentIterationUseCase::new(telemetry),
        }
    }
}
