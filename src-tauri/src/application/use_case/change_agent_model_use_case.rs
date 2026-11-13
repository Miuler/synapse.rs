use crate::application::dto::agent::ChangeAgentModelCommand;
use crate::application::integration::service::AgentRunner;

/// Cambia el modelo que usa el agente en la sesión.
#[derive(Clone)]
pub struct ChangeAgentModelUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> ChangeAgentModelUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: ChangeAgentModelCommand) -> Result<(), String> {
        self.runner
            .set_model(&command.session_id, &command.model_id)
            .await
    }
}
