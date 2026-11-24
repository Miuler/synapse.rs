use crate::application::dto::agent::ChangeAgentOptionCommand;
use crate::application::integration::service::AgentRunner;
use crate::domain::agent::AgentConfig;

/// Cambia una opción de configuración de la sesión (modelo, esfuerzo) y devuelve cómo quedó.
#[derive(Clone)]
pub struct ChangeAgentOptionUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> ChangeAgentOptionUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: ChangeAgentOptionCommand) -> Result<AgentConfig, String> {
        self.runner
            .set_option(&command.session_id, &command.option_id, &command.value)
            .await
    }
}
