use crate::application::dto::session::get_note_view_mode_query::GetNoteViewModeQuery;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Modo de vista recordado de una nota.
#[derive(Clone)]
pub struct GetNoteViewModeUseCase<A: WorkspaceSessionRepository> {
    workspace_session_repository: A,
}

impl<A: WorkspaceSessionRepository> GetNoteViewModeUseCase<A> {
    pub fn new(workspace_session_repository: A) -> Self {
        Self {
            workspace_session_repository,
        }
    }

    pub fn execute(&self, query: GetNoteViewModeQuery) -> Result<Option<String>, String> {
        self.workspace_session_repository
            .view_mode(&query.relative_path)
    }
}
