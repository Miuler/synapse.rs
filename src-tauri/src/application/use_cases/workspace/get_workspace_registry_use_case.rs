use crate::application::dto::workspace::get_workspace_registry_query::GetWorkspaceRegistryQuery;
use crate::domain::repository::workspace_registry_repository::WorkspaceRegistryRepository;
use crate::domain::model::workspace_registry::WorkspaceRegistryConfig;

/// Registro de espacios de trabajo.
#[derive(Clone)]
pub struct GetWorkspaceRegistryUseCase<A: WorkspaceRegistryRepository> {
    workspaces: A,
}

impl<A: WorkspaceRegistryRepository> GetWorkspaceRegistryUseCase<A> {
    pub fn new(workspaces: A) -> Self {
        Self { workspaces }
    }

    pub fn execute(&self, _query: GetWorkspaceRegistryQuery) -> Result<WorkspaceRegistryConfig, String> {
        self.workspaces.load_registry()
    }
}
