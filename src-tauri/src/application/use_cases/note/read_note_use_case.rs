use crate::application::dto::note::read_note_query::ReadNoteQuery;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::note::note::Note;
use crate::domain::note::note_path::NoteRelativePath;

/// Lee una nota de la bóveda.
#[derive(Clone)]
pub struct ReadNoteUseCase<A: NoteRepository> {
    notes: A,
}

impl<A: NoteRepository> ReadNoteUseCase<A> {
    pub fn new(notes: A) -> Self {
        Self { notes }
    }

    pub fn execute(&self, query: ReadNoteQuery) -> Result<Note, String> {
        let relative_path = NoteRelativePath::new(&query.relative_path)?;
        self.notes.read_note(&query.vault_path, &relative_path)
    }
}
