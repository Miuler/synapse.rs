use crate::application::dto::session::get_open_tabs_query::GetOpenTabsQuery;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;
use crate::domain::workspace::workspace_session::WorkspaceOpenTabsState;

/// Pestañas abiertas guardadas.
#[derive(Clone)]
pub struct GetOpenTabsUseCase<A: WorkspaceSessionRepository> {
    sessions: A,
}

impl<A: WorkspaceSessionRepository> GetOpenTabsUseCase<A> {
    pub fn new(sessions: A) -> Self {
        Self { sessions }
    }

    pub fn execute(&self, _query: GetOpenTabsQuery) -> Result<WorkspaceOpenTabsState, String> {
        self.sessions.open_tabs()
    }
}
