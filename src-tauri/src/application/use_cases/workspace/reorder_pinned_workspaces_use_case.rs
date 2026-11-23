use crate::application::dto::workspace::reorder_pinned_workspaces_command::ReorderPinnedWorkspacesCommand;
use crate::domain::workspace::workspace_registry::RecentWorkspaceEntry;
use crate::domain::workspace::workspace_registry_repository::WorkspaceRegistryRepository;

/// Reordena los espacios de trabajo fijados según el orden especificado.
#[derive(Clone)]
pub struct ReorderPinnedWorkspacesUseCase<A: WorkspaceRegistryRepository> {
    workspace_registry_repository: A,
}

impl<A: WorkspaceRegistryRepository> ReorderPinnedWorkspacesUseCase<A> {
    pub fn new(workspace_registry_repository: A) -> Self {
        Self {
            workspace_registry_repository,
        }
    }

    pub fn execute(
        &self,
        command: ReorderPinnedWorkspacesCommand,
    ) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.workspace_registry_repository
            .reorder_pinned_workspaces(&command.paths)
    }
}
