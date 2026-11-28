use crate::application::dto::agent::SendAgentPromptCommand;
use crate::application::integration::service::AgentRunner;

/// Envía un mensaje al agente y espera a que termine su turno.
#[derive(Clone)]
pub struct SendAgentPromptUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> SendAgentPromptUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: SendAgentPromptCommand) -> Result<String, String> {
        let blocks: Vec<_> = command
            .blocks
            .into_iter()
            .filter(|b| !b.is_empty())
            .collect();
        if blocks.is_empty() {
            return Err("message is empty".into());
        }
        self.runner.prompt(&command.session_id, blocks).await
    }
}
