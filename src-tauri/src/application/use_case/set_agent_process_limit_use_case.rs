use crate::application::dto::agent::SetAgentProcessLimitCommand;
use crate::application::integration::service::AgentRunner;

/// Cuántas conversaciones comparte un proceso de un agente (1 = una por proceso).
#[derive(Clone)]
pub struct SetAgentProcessLimitUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> SetAgentProcessLimitUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub fn execute(&self, command: SetAgentProcessLimitCommand) {
        self.runner
            .set_process_limit(&command.agent_id, command.limit.clamp(1, 16));
    }
}
