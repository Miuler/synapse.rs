use crate::application::dto::agent::SaveAgentProfileCommand;
use crate::domain::agent::{AgentProfile, AgentProfileRepository};

/// Guarda un agente personalizado (nuevo o editado).
#[derive(Clone)]
pub struct SaveAgentProfileUseCase<R: AgentProfileRepository> {
    profiles: R,
}

impl<R: AgentProfileRepository> SaveAgentProfileUseCase<R> {
    pub fn new(profiles: R) -> Self {
        Self { profiles }
    }

    pub fn execute(&self, command: SaveAgentProfileCommand) -> Result<AgentProfile, String> {
        let profile = AgentProfile::custom(&command.id, &command.name, command.launch)?;
        self.profiles.save(profile.clone())?;
        Ok(profile)
    }
}
