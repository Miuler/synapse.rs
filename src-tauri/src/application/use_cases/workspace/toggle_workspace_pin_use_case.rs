use crate::application::dto::workspace::toggle_workspace_pin_command::ToggleWorkspacePinCommand;
use crate::domain::workspace::workspace_registry_repository::WorkspaceRegistryRepository;
use crate::domain::workspace::workspace_registry::RecentWorkspaceEntry;

/// Fija o desfija un espacio de trabajo.
#[derive(Clone)]
pub struct ToggleWorkspacePinUseCase<A: WorkspaceRegistryRepository> {
    workspaces: A,
}

impl<A: WorkspaceRegistryRepository> ToggleWorkspacePinUseCase<A> {
    pub fn new(workspaces: A) -> Self {
        Self { workspaces }
    }

    pub fn execute(&self, command: ToggleWorkspacePinCommand) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.workspaces.toggle_pin(&command.path)
    }
}
