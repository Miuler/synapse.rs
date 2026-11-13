use crate::application::dto::agent::CancelAgentPromptCommand;
use crate::application::integration::service::AgentRunner;

/// Detiene el turno en curso del agente sin cerrar la sesión.
#[derive(Clone)]
pub struct CancelAgentPromptUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> CancelAgentPromptUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: CancelAgentPromptCommand) -> Result<(), String> {
        self.runner.cancel(&command.session_id).await
    }
}
