use crate::application::dto::session::get_open_tabs_query::GetOpenTabsQuery;
use crate::domain::workspace::workspace_session::WorkspaceOpenTabsState;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Pestañas abiertas guardadas.
#[derive(Clone)]
pub struct GetOpenTabsUseCase<A: WorkspaceSessionRepository> {
    workspace_session_repository: A,
}

impl<A: WorkspaceSessionRepository> GetOpenTabsUseCase<A> {
    pub fn new(workspace_session_repository: A) -> Self {
        Self {
            workspace_session_repository,
        }
    }

    pub fn execute(&self, _query: GetOpenTabsQuery) -> Result<WorkspaceOpenTabsState, String> {
        self.workspace_session_repository.open_tabs()
    }
}
