use crate::application::dto::agent::ChangeAgentModeCommand;
use crate::application::integration::service::AgentRunner;

/// Cambia el modo de trabajo del agente (por ejemplo, planificar o editar).
#[derive(Clone)]
pub struct ChangeAgentModeUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> ChangeAgentModeUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: ChangeAgentModeCommand) -> Result<(), String> {
        self.runner
            .set_mode(&command.session_id, &command.mode_id)
            .await
    }
}
