use crate::application::dto::note::read_standalone_note_query::ReadStandaloneNoteQuery;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::note::note::Note;

/// Lee un archivo suelto, fuera de una bóveda.
#[derive(Clone)]
pub struct ReadStandaloneNoteUseCase<A: NoteRepository> {
    notes: A,
}

impl<A: NoteRepository> ReadStandaloneNoteUseCase<A> {
    pub fn new(notes: A) -> Self {
        Self { notes }
    }

    pub fn execute(&self, query: ReadStandaloneNoteQuery) -> Result<Note, String> {
        self.notes.read_standalone_file(&query.path)
    }
}
