use crate::application::dto::session::get_recent_notes_query::GetRecentNotesQuery;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Rutas de las notas abiertas más recientemente.
#[derive(Clone)]
pub struct GetRecentNotesUseCase<A: WorkspaceSessionRepository> {
    sessions: A,
}

impl<A: WorkspaceSessionRepository> GetRecentNotesUseCase<A> {
    pub fn new(sessions: A) -> Self {
        Self { sessions }
    }

    pub fn execute(&self, query: GetRecentNotesQuery) -> Result<Vec<String>, String> {
        self.sessions.recent_note_paths(query.limit)
    }
}
