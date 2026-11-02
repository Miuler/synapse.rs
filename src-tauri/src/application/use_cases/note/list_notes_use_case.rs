use crate::application::dto::note::list_notes_query::ListNotesQuery;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::note::note::Note;
use log::debug;

/// Lista las notas de la bóveda con las extensiones indicadas.
#[derive(Clone)]
pub struct ListNotesUseCase<A: NoteRepository> {
    notes: A,
}

impl<A: NoteRepository> ListNotesUseCase<A> {
    pub fn new(notes: A) -> Self {
        Self { notes }
    }

    pub fn execute(&self, query: ListNotesQuery) -> Result<Vec<Note>, String> {
        debug!(
            "list_notes in: {:?} with extensions: {:?}",
            query.vault_path, query.extensions
        );
        let notes = self.notes.list_notes(&query.vault_path, &query.extensions)?;
        debug!("notes.len: {}", notes.len());
        Ok(notes)
    }
}
