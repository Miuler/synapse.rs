use crate::application::dto::note::save_note_command::SaveNoteCommand;
use crate::domain::note::note::Note;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;

/// Guarda una nota de la bóveda con su codificación.
#[derive(Clone)]
pub struct SaveNoteUseCase<A: NoteRepository> {
    note_repository: A,
}

impl<A: NoteRepository> SaveNoteUseCase<A> {
    pub fn new(note_repository: A) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, command: SaveNoteCommand) -> Result<(), String> {
        let relative_path = NoteRelativePath::new(&command.relative_path)?;
        let note = Note::with_encoding(
            relative_path,
            "".to_string(),
            command.title,
            command.content,
            command.encoding,
        );
        self.note_repository.save_note(&command.vault_path, &note)
    }
}
