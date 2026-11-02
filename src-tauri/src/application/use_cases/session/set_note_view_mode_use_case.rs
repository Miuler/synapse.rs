use crate::application::dto::session::set_note_view_mode_command::SetNoteViewModeCommand;
use crate::domain::repository::workspace_session_repository::WorkspaceSessionRepository;

/// Recuerda el modo de vista de una nota.
#[derive(Clone)]
pub struct SetNoteViewModeUseCase<A: WorkspaceSessionRepository> {
    sessions: A,
}

impl<A: WorkspaceSessionRepository> SetNoteViewModeUseCase<A> {
    pub fn new(sessions: A) -> Self {
        Self { sessions }
    }

    pub fn execute(&self, command: SetNoteViewModeCommand) -> Result<(), String> {
        self.sessions.set_view_mode(&command.relative_path, &command.view_mode)
    }
}
