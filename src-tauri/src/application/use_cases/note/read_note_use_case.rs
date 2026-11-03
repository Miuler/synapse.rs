use crate::application::dto::note::read_note_query::ReadNoteQuery;
use crate::domain::note::note::Note;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;

/// Lee una nota de la bóveda.
#[derive(Clone)]
pub struct ReadNoteUseCase<A: NoteRepository> {
    note_repository: A,
}

impl<A: NoteRepository> ReadNoteUseCase<A> {
    pub fn new(note_repository: A) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, query: ReadNoteQuery) -> Result<Note, String> {
        let relative_path = NoteRelativePath::new(&query.relative_path)?;
        self.note_repository
            .read_note(&query.vault_path, &relative_path)
    }
}
