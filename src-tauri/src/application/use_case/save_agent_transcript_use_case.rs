use crate::application::dto::agent::SaveAgentTranscriptCommand;
use crate::domain::agent::{AgentTranscript, AgentTranscriptRepository};

/// Guarda (o actualiza) una conversación con un agente.
#[derive(Clone)]
pub struct SaveAgentTranscriptUseCase<R: AgentTranscriptRepository> {
    transcripts: R,
}

impl<R: AgentTranscriptRepository> SaveAgentTranscriptUseCase<R> {
    pub fn new(transcripts: R) -> Self {
        Self { transcripts }
    }

    pub fn execute(&self, command: SaveAgentTranscriptCommand) -> Result<(), String> {
        if !AgentTranscript::valid_id(&command.transcript.id) {
            return Err("Identificador de conversación inválido".into());
        }
        self.transcripts.save(command.transcript)
    }
}
