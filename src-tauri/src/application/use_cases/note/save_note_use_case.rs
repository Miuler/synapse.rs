use crate::application::dto::note::save_note_command::SaveNoteCommand;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::note::note::Note;
use crate::domain::note::note_path::NoteRelativePath;

/// Guarda una nota de la bóveda con su codificación.
#[derive(Clone)]
pub struct SaveNoteUseCase<A: NoteRepository> {
    notes: A,
}

impl<A: NoteRepository> SaveNoteUseCase<A> {
    pub fn new(notes: A) -> Self {
        Self { notes }
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
        self.notes.save_note(&command.vault_path, &note)
    }
}
