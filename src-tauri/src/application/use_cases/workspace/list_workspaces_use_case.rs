use crate::application::dto::workspace::list_workspaces_query::ListWorkspacesQuery;
use crate::domain::repository::workspace_registry_repository::WorkspaceRegistryRepository;
use crate::domain::model::workspace_registry::RecentWorkspaceEntry;

/// Espacios de trabajo recientes.
#[derive(Clone)]
pub struct ListWorkspacesUseCase<A: WorkspaceRegistryRepository> {
    workspaces: A,
}

impl<A: WorkspaceRegistryRepository> ListWorkspacesUseCase<A> {
    pub fn new(workspaces: A) -> Self {
        Self { workspaces }
    }

    pub fn execute(&self, _query: ListWorkspacesQuery) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.workspaces.list_workspaces()
    }
}
