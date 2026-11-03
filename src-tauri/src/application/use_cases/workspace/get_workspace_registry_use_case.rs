use crate::application::dto::workspace::get_workspace_registry_query::GetWorkspaceRegistryQuery;
use crate::domain::workspace::workspace_registry::WorkspaceRegistryConfig;
use crate::domain::workspace::workspace_registry_repository::WorkspaceRegistryRepository;

/// Registro de espacios de trabajo.
#[derive(Clone)]
pub struct GetWorkspaceRegistryUseCase<A: WorkspaceRegistryRepository> {
    workspace_registry_repository: A,
}

impl<A: WorkspaceRegistryRepository> GetWorkspaceRegistryUseCase<A> {
    pub fn new(workspace_registry_repository: A) -> Self {
        Self {
            workspace_registry_repository,
        }
    }

    pub fn execute(
        &self,
        _query: GetWorkspaceRegistryQuery,
    ) -> Result<WorkspaceRegistryConfig, String> {
        self.workspace_registry_repository.load_registry()
    }
}
