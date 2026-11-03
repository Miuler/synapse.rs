use crate::application::dto::workspace::toggle_workspace_pin_command::ToggleWorkspacePinCommand;
use crate::domain::workspace::workspace_registry::RecentWorkspaceEntry;
use crate::domain::workspace::workspace_registry_repository::WorkspaceRegistryRepository;

/// Fija o desfija un espacio de trabajo.
#[derive(Clone)]
pub struct ToggleWorkspacePinUseCase<A: WorkspaceRegistryRepository> {
    workspace_registry_repository: A,
}

impl<A: WorkspaceRegistryRepository> ToggleWorkspacePinUseCase<A> {
    pub fn new(workspace_registry_repository: A) -> Self {
        Self {
            workspace_registry_repository,
        }
    }

    pub fn execute(
        &self,
        command: ToggleWorkspacePinCommand,
    ) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.workspace_registry_repository.toggle_pin(&command.path)
    }
}
