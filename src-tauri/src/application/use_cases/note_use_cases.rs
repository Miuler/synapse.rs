use crate::domain::models::note::Note;
use crate::domain::repositories::note_repository::NoteRepository;
use crate::domain::value_objects::note_path::NoteRelativePath;
use std::path::Path;
use log::info;

/// Casos de uso de la aplicación para gestionar notas.
pub struct NoteUseCases<R: NoteRepository> {
    repository: R,
}

impl<R: NoteRepository> NoteUseCases<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn list_notes(&self, vault_path: &Path, extensions: &[String]) -> Result<Vec<Note>, String> {
        info!("list_notes in: {:?} with extensions: {:?}", vault_path, extensions);
        self.repository.list_notes(vault_path, extensions)
    }

    pub fn read_note(&self, vault_path: &Path, relative_path_str: &str) -> Result<Note, String> {
        let relative_path = NoteRelativePath::new(relative_path_str)?;
        self.repository.read_note(vault_path, &relative_path)
    }

    pub fn save_note(&self, vault_path: &Path, relative_path_str: &str, title: &str, content: &str) -> Result<(), String> {
        let relative_path = NoteRelativePath::new(relative_path_str)?;
        let note = Note::new(relative_path, title.to_string(), content.to_string());
        self.repository.save_note(vault_path, &note)
    }
}
