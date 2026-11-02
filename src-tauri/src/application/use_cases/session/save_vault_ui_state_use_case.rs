use crate::application::dto::session::save_vault_ui_state_command::SaveVaultUiStateCommand;
use crate::domain::repository::workspace_session_repository::WorkspaceSessionRepository;

/// Guarda el estado de la interfaz de la bóveda.
#[derive(Clone)]
pub struct SaveVaultUiStateUseCase<A: WorkspaceSessionRepository> {
    sessions: A,
}

impl<A: WorkspaceSessionRepository> SaveVaultUiStateUseCase<A> {
    pub fn new(sessions: A) -> Self {
        Self { sessions }
    }

    pub fn execute(&self, command: SaveVaultUiStateCommand) -> Result<(), String> {
        self.sessions.save_ui_state(
            command.sidebar_width,
            command.expanded_folders,
            command.diff_visual_mode,
            command.diff_text_mode,
            command.locked_files,
        )
    }
}
