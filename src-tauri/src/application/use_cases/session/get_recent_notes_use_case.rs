use crate::application::dto::session::get_recent_notes_query::GetRecentNotesQuery;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Rutas de las notas abiertas más recientemente.
#[derive(Clone)]
pub struct GetRecentNotesUseCase<A: WorkspaceSessionRepository> {
    workspace_session_repository: A,
}

impl<A: WorkspaceSessionRepository> GetRecentNotesUseCase<A> {
    pub fn new(workspace_session_repository: A) -> Self {
        Self {
            workspace_session_repository,
        }
    }

    pub fn execute(&self, query: GetRecentNotesQuery) -> Result<Vec<String>, String> {
        self.workspace_session_repository
            .recent_note_paths(query.limit)
    }
}
