use super::{AgentTranscript, AgentTranscriptSummary};

/// Conversaciones guardadas con los agentes, dentro de cada bóveda (`cwd`).
pub trait AgentTranscriptRepository: Send + Sync {
    fn save(&self, transcript: AgentTranscript) -> Result<(), String>;
    /// Conversaciones de una bóveda, la más reciente primero.
    fn list(&self, cwd: &str) -> Result<Vec<AgentTranscriptSummary>, String>;
    fn load(&self, cwd: &str, id: &str) -> Result<AgentTranscript, String>;
    fn delete(&self, cwd: &str, id: &str) -> Result<(), String>;
    /// Borra todas las conversaciones de la bóveda.
    fn delete_all(&self, cwd: &str) -> Result<(), String>;
    /// Carpeta y nombre del archivo de la conversación, para abrirla fuera de Synapse.
    fn location(&self, cwd: &str, id: &str) -> Result<(std::path::PathBuf, String), String>;
}
