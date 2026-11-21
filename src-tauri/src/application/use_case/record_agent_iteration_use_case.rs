use crate::application::dto::agent::RecordAgentIterationCommand;
use crate::domain::agent::AgentTelemetryRepository;

pub struct RecordAgentIterationUseCase<R: AgentTelemetryRepository> {
    repository: R,
}

impl<R: AgentTelemetryRepository> RecordAgentIterationUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&self, cmd: RecordAgentIterationCommand) -> Result<(), String> {
        self.repository.record_iteration(
            &cmd.cwd,
            &cmd.conversation_id,
            &cmd.agent_id,
            &cmd.agent_name,
            cmd.acp_session_id.as_deref(),
            cmd.iteration,
        )
    }
}
