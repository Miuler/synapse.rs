use crate::application::dto::agent::StartAgentSessionCommand;
use crate::application::integration::service::{AgentInstaller, AgentRunner};
use crate::domain::agent::{AgentProfileRepository, AgentSession};

/// Abre una conversación con un agente: si es verificado lo instala antes si hace falta.
#[derive(Clone)]
pub struct StartAgentSessionUseCase<R: AgentProfileRepository, I: AgentInstaller, A: AgentRunner> {
    profiles: R,
    installer: I,
    runner: A,
}

impl<R: AgentProfileRepository, I: AgentInstaller, A: AgentRunner>
    StartAgentSessionUseCase<R, I, A>
{
    pub fn new(profiles: R, installer: I, runner: A) -> Self {
        Self {
            profiles,
            installer,
            runner,
        }
    }

    pub async fn execute(&self, command: StartAgentSessionCommand) -> Result<AgentSession, String> {
        let custom = self
            .profiles
            .list()?
            .into_iter()
            .find(|p| p.id == command.agent_id);
        let launch = match custom.and_then(|p| p.launch) {
            Some(launch) => launch,
            None => self.installer.ensure_installed(&command.agent_id).await?,
        };
        self.runner
            .start(
                &command.agent_id,
                launch,
                &command.cwd,
                command.tools,
                command.instructions,
                command.resume,
                command.fork,
            )
            .await
    }
}
