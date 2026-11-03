use crate::application::dto::workspace::list_workspaces_query::ListWorkspacesQuery;
use crate::domain::workspace::workspace_registry::RecentWorkspaceEntry;
use crate::domain::workspace::workspace_registry_repository::WorkspaceRegistryRepository;

/// Espacios de trabajo recientes.
#[derive(Clone)]
pub struct ListWorkspacesUseCase<A: WorkspaceRegistryRepository> {
    workspace_registry_repository: A,
}

impl<A: WorkspaceRegistryRepository> ListWorkspacesUseCase<A> {
    pub fn new(workspace_registry_repository: A) -> Self {
        Self {
            workspace_registry_repository,
        }
    }

    pub fn execute(
        &self,
        _query: ListWorkspacesQuery,
    ) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.workspace_registry_repository.list_workspaces()
    }
}
