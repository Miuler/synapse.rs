use crate::application::dto::agent::RespondElicitationCommand;
use crate::application::integration::service::AgentRunner;

/// Devuelve al agente la respuesta del usuario a una solicitud de elicitación (`elicitation/create`).
#[derive(Clone)]
pub struct RespondElicitationUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> RespondElicitationUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: RespondElicitationCommand) -> Result<(), String> {
        self.runner
            .respond_elicitation(&command.session_id, &command.request_id, command.decision)
            .await
    }
}
