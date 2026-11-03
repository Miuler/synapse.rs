use crate::application::dto::note::list_notes_query::ListNotesQuery;
use crate::domain::note::note::Note;
use crate::domain::note::note_repository::NoteRepository;
use log::debug;

/// Lista las notas de la bóveda con las extensiones indicadas.
#[derive(Clone)]
pub struct ListNotesUseCase<A: NoteRepository> {
    note_repository: A,
}

impl<A: NoteRepository> ListNotesUseCase<A> {
    pub fn new(note_repository: A) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, query: ListNotesQuery) -> Result<Vec<Note>, String> {
        debug!(
            "list_notes in: {:?} with extensions: {:?}",
            query.vault_path, query.extensions
        );
        let notes = self
            .note_repository
            .list_notes(&query.vault_path, &query.extensions)?;
        debug!("notes.len: {}", notes.len());
        Ok(notes)
    }
}
