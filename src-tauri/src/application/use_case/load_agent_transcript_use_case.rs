use crate::application::dto::agent::LoadAgentTranscriptQuery;
use crate::domain::agent::{AgentTranscript, AgentTranscriptRepository};

/// Lee una conversación guardada completa.
#[derive(Clone)]
pub struct LoadAgentTranscriptUseCase<R: AgentTranscriptRepository> {
    transcripts: R,
}

impl<R: AgentTranscriptRepository> LoadAgentTranscriptUseCase<R> {
    pub fn new(transcripts: R) -> Self {
        Self { transcripts }
    }

    pub fn execute(&self, query: LoadAgentTranscriptQuery) -> Result<AgentTranscript, String> {
        self.transcripts.load(&query.cwd, &query.id)
    }
}
