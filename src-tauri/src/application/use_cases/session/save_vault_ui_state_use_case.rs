use crate::application::dto::session::save_vault_ui_state_command::SaveVaultUiStateCommand;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Guarda el estado de la interfaz de la bóveda.
#[derive(Clone)]
pub struct SaveVaultUiStateUseCase<A: WorkspaceSessionRepository> {
    workspace_session_repository: A,
}

impl<A: WorkspaceSessionRepository> SaveVaultUiStateUseCase<A> {
    pub fn new(workspace_session_repository: A) -> Self {
        Self {
            workspace_session_repository,
        }
    }

    pub fn execute(&self, command: SaveVaultUiStateCommand) -> Result<(), String> {
        self.workspace_session_repository.save_ui_state(
            command.sidebar_width,
            command.expanded_folders,
            command.diff_visual_mode,
            command.diff_text_mode,
            command.locked_files,
        )
    }
}
