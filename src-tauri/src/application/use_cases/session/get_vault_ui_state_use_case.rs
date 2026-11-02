use crate::application::dto::session::get_vault_ui_state_query::GetVaultUiStateQuery;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;
use crate::domain::workspace::workspace_session::VaultUiState;

/// Estado de la interfaz de la bóveda.
#[derive(Clone)]
pub struct GetVaultUiStateUseCase<A: WorkspaceSessionRepository> {
    sessions: A,
}

impl<A: WorkspaceSessionRepository> GetVaultUiStateUseCase<A> {
    pub fn new(sessions: A) -> Self {
        Self { sessions }
    }

    pub fn execute(&self, _query: GetVaultUiStateQuery) -> Result<VaultUiState, String> {
        self.sessions.ui_state()
    }
}
