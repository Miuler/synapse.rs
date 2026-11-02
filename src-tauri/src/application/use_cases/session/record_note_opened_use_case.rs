use crate::application::dto::session::record_note_opened_command::RecordNoteOpenedCommand;
use crate::domain::repository::workspace_session_repository::WorkspaceSessionRepository;

/// Registra que se abrió una nota (para recientes y prioridad del Quick Open).
#[derive(Clone)]
pub struct RecordNoteOpenedUseCase<A: WorkspaceSessionRepository> {
    sessions: A,
}

impl<A: WorkspaceSessionRepository> RecordNoteOpenedUseCase<A> {
    pub fn new(sessions: A) -> Self {
        Self { sessions }
    }

    pub fn execute(&self, command: RecordNoteOpenedCommand) -> Result<(), String> {
        self.sessions.record_opened(&command.relative_path)
    }
}
