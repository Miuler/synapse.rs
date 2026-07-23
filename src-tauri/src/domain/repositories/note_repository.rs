use crate::domain::models::note::Note;
use crate::domain::value_objects::note_path::NoteRelativePath;
use std::path::Path;

/// Puerto (Trait) del repositorio de notas en el Dominio.
pub trait NoteRepository: Send + Sync {
    fn list_notes(&self, vault_path: &Path) -> Result<Vec<Note>, String>;
    fn read_note(&self, vault_path: &Path, relative_path: &NoteRelativePath) -> Result<Note, String>;
    fn save_note(&self, vault_path: &Path, note: &Note) -> Result<(), String>;
}
