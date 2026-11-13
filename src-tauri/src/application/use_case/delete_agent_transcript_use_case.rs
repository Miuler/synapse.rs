use crate::application::dto::agent::DeleteAgentTranscriptCommand;
use crate::domain::agent::AgentTranscriptRepository;

/// Borra una conversación guardada.
#[derive(Clone)]
pub struct DeleteAgentTranscriptUseCase<R: AgentTranscriptRepository> {
    transcripts: R,
}

impl<R: AgentTranscriptRepository> DeleteAgentTranscriptUseCase<R> {
    pub fn new(transcripts: R) -> Self {
        Self { transcripts }
    }

    pub fn execute(&self, command: DeleteAgentTranscriptCommand) -> Result<(), String> {
        self.transcripts.delete(&command.cwd, &command.id)
    }
}
