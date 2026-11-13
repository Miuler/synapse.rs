use crate::application::dto::agent::InstallAgentCommand;
use crate::application::integration::service::AgentInstaller;

/// Descarga e instala un agente verificado.
#[derive(Clone)]
pub struct InstallAgentUseCase<I: AgentInstaller> {
    installer: I,
}

impl<I: AgentInstaller> InstallAgentUseCase<I> {
    pub fn new(installer: I) -> Self {
        Self { installer }
    }

    pub async fn execute(&self, command: InstallAgentCommand) -> Result<(), String> {
        self.installer
            .ensure_installed(&command.agent_id)
            .await
            .map(|_| ())
    }
}
