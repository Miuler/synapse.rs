use crate::application::dto::session::set_note_view_mode_command::SetNoteViewModeCommand;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Recuerda el modo de vista de una nota.
#[derive(Clone)]
pub struct SetNoteViewModeUseCase<A: WorkspaceSessionRepository> {
    workspace_session_repository: A,
}

impl<A: WorkspaceSessionRepository> SetNoteViewModeUseCase<A> {
    pub fn new(workspace_session_repository: A) -> Self {
        Self {
            workspace_session_repository,
        }
    }

    pub fn execute(&self, command: SetNoteViewModeCommand) -> Result<(), String> {
        self.workspace_session_repository
            .set_view_mode(&command.relative_path, &command.view_mode)
    }
}
