use crate::application::dto::agent::DeleteAllAgentTranscriptsCommand;
use crate::domain::agent::AgentTranscriptRepository;

/// Borra todas las conversaciones guardadas de una bóveda.
#[derive(Clone)]
pub struct DeleteAllAgentTranscriptsUseCase<R: AgentTranscriptRepository> {
    transcripts: R,
}

impl<R: AgentTranscriptRepository> DeleteAllAgentTranscriptsUseCase<R> {
    pub fn new(transcripts: R) -> Self {
        Self { transcripts }
    }

    pub fn execute(&self, command: DeleteAllAgentTranscriptsCommand) -> Result<(), String> {
        self.transcripts.delete_all(&command.cwd)
    }
}
