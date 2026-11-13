use crate::application::dto::agent::DeleteAgentProfileCommand;
use crate::domain::agent::AgentProfileRepository;

/// Borra un agente personalizado.
#[derive(Clone)]
pub struct DeleteAgentProfileUseCase<R: AgentProfileRepository> {
    profiles: R,
}

impl<R: AgentProfileRepository> DeleteAgentProfileUseCase<R> {
    pub fn new(profiles: R) -> Self {
        Self { profiles }
    }

    pub fn execute(&self, command: DeleteAgentProfileCommand) -> Result<(), String> {
        self.profiles.delete(&command.id)
    }
}
