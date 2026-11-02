use crate::application::dto::workspace::record_workspace_opened_command::RecordWorkspaceOpenedCommand;
use crate::domain::repository::workspace_registry_repository::WorkspaceRegistryRepository;
use crate::domain::model::workspace_registry::RecentWorkspaceEntry;

/// Registra que se abrió un espacio de trabajo.
#[derive(Clone)]
pub struct RecordWorkspaceOpenedUseCase<A: WorkspaceRegistryRepository> {
    workspaces: A,
}

impl<A: WorkspaceRegistryRepository> RecordWorkspaceOpenedUseCase<A> {
    pub fn new(workspaces: A) -> Self {
        Self { workspaces }
    }

    pub fn execute(&self, command: RecordWorkspaceOpenedCommand) -> Result<RecentWorkspaceEntry, String> {
        self.workspaces
            .record_workspace_opened(&command.path, command.notes_count, command.git_branch, command.is_git_repo)
    }
}
