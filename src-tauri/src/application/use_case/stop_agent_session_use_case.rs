use crate::application::dto::agent::StopAgentSessionCommand;
use crate::application::integration::service::AgentRunner;

/// Cierra la conversación y termina el proceso del agente.
#[derive(Clone)]
pub struct StopAgentSessionUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> StopAgentSessionUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: StopAgentSessionCommand) -> Result<(), String> {
        self.runner.stop(&command.session_id).await
    }
}
