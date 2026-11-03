use crate::application::dto::note::read_standalone_note_query::ReadStandaloneNoteQuery;
use crate::domain::note::note::Note;
use crate::domain::note::note_repository::NoteRepository;

/// Lee un archivo suelto, fuera de una bóveda.
#[derive(Clone)]
pub struct ReadStandaloneNoteUseCase<A: NoteRepository> {
    note_repository: A,
}

impl<A: NoteRepository> ReadStandaloneNoteUseCase<A> {
    pub fn new(note_repository: A) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, query: ReadStandaloneNoteQuery) -> Result<Note, String> {
        self.note_repository.read_standalone_file(&query.path)
    }
}
