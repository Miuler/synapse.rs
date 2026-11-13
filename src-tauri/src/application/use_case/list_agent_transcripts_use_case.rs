use crate::application::dto::agent::ListAgentTranscriptsQuery;
use crate::domain::agent::{AgentTranscriptRepository, AgentTranscriptSummary};

/// Conversaciones guardadas de una bóveda, la más reciente primero.
#[derive(Clone)]
pub struct ListAgentTranscriptsUseCase<R: AgentTranscriptRepository> {
    transcripts: R,
}

impl<R: AgentTranscriptRepository> ListAgentTranscriptsUseCase<R> {
    pub fn new(transcripts: R) -> Self {
        Self { transcripts }
    }

    pub fn execute(
        &self,
        query: ListAgentTranscriptsQuery,
    ) -> Result<Vec<AgentTranscriptSummary>, String> {
        self.transcripts.list(&query.cwd)
    }
}
