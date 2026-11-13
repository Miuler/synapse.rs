use crate::application::dto::agent::RespondPermissionCommand;
use crate::application::integration::service::AgentRunner;

/// Devuelve al agente la decisión del usuario sobre un permiso.
#[derive(Clone)]
pub struct RespondPermissionUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> RespondPermissionUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(&self, command: RespondPermissionCommand) -> Result<(), String> {
        self.runner
            .respond_permission(&command.session_id, &command.request_id, command.decision)
            .await
    }
}
