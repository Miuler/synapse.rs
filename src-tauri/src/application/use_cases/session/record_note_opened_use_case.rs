use crate::application::dto::session::record_note_opened_command::RecordNoteOpenedCommand;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Registra que se abrió una nota (para recientes y prioridad del Quick Open).
#[derive(Clone)]
pub struct RecordNoteOpenedUseCase<A: WorkspaceSessionRepository> {
    workspace_session_repository: A,
}

impl<A: WorkspaceSessionRepository> RecordNoteOpenedUseCase<A> {
    pub fn new(workspace_session_repository: A) -> Self {
        Self {
            workspace_session_repository,
        }
    }

    pub fn execute(&self, command: RecordNoteOpenedCommand) -> Result<(), String> {
        self.workspace_session_repository
            .record_opened(&command.relative_path)
    }
}
