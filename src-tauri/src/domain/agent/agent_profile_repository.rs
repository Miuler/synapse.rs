use super::AgentProfile;

/// Agentes personalizados que define el usuario.
pub trait AgentProfileRepository: Send + Sync {
    fn list(&self) -> Result<Vec<AgentProfile>, String>;
    fn save(&self, profile: AgentProfile) -> Result<(), String>;
    fn delete(&self, id: &str) -> Result<(), String>;
}
