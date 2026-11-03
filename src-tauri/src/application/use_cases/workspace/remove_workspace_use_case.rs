use crate::application::dto::workspace::remove_workspace_command::RemoveWorkspaceCommand;
use crate::domain::workspace::workspace_registry::RecentWorkspaceEntry;
use crate::domain::workspace::workspace_registry_repository::WorkspaceRegistryRepository;

/// Quita un espacio de trabajo de la lista de recientes.
#[derive(Clone)]
pub struct RemoveWorkspaceUseCase<A: WorkspaceRegistryRepository> {
    workspace_registry_repository: A,
}

impl<A: WorkspaceRegistryRepository> RemoveWorkspaceUseCase<A> {
    pub fn new(workspace_registry_repository: A) -> Self {
        Self {
            workspace_registry_repository,
        }
    }

    pub fn execute(
        &self,
        command: RemoveWorkspaceCommand,
    ) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.workspace_registry_repository
            .remove_workspace(&command.path)
    }
}
