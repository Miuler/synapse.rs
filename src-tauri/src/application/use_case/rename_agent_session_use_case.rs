use crate::application::dto::agent::RenameAgentSessionCommand;
use crate::application::integration::service::AgentRunner;

/// Cambia el nombre de la sesión en el propio agente (no solo en Synapse).
#[derive(Clone)]
pub struct RenameAgentSessionUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> RenameAgentSessionUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: RenameAgentSessionCommand) -> Result<String, String> {
        let title = command.title.trim();
        if title.is_empty() {
            return Err("session name cannot be empty".into());
        }
        self.runner.set_title(&command.session_id, title).await
    }
}
