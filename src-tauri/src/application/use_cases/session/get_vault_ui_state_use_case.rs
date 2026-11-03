use crate::application::dto::session::get_vault_ui_state_query::GetVaultUiStateQuery;
use crate::domain::workspace::workspace_session::VaultUiState;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Estado de la interfaz de la bóveda.
#[derive(Clone)]
pub struct GetVaultUiStateUseCase<A: WorkspaceSessionRepository> {
    workspace_session_repository: A,
}

impl<A: WorkspaceSessionRepository> GetVaultUiStateUseCase<A> {
    pub fn new(workspace_session_repository: A) -> Self {
        Self {
            workspace_session_repository,
        }
    }

    pub fn execute(&self, _query: GetVaultUiStateQuery) -> Result<VaultUiState, String> {
        self.workspace_session_repository.ui_state()
    }
}
