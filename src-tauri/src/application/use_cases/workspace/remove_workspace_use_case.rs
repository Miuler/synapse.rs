use crate::application::dto::workspace::remove_workspace_command::RemoveWorkspaceCommand;
use crate::domain::repository::workspace_registry_repository::WorkspaceRegistryRepository;
use crate::domain::model::workspace_registry::RecentWorkspaceEntry;

/// Quita un espacio de trabajo de la lista de recientes.
#[derive(Clone)]
pub struct RemoveWorkspaceUseCase<A: WorkspaceRegistryRepository> {
    workspaces: A,
}

impl<A: WorkspaceRegistryRepository> RemoveWorkspaceUseCase<A> {
    pub fn new(workspaces: A) -> Self {
        Self { workspaces }
    }

    pub fn execute(&self, command: RemoveWorkspaceCommand) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.workspaces.remove_workspace(&command.path)
    }
}
