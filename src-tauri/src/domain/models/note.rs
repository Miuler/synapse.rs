use crate::domain::value_objects::note_path::NoteRelativePath;
use serde::{Deserialize, Serialize};

/// Entidad Note que representa una nota dentro del dominio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub relative_path: NoteRelativePath,
    pub title: String,
    pub content: String,
}

impl Note {
    pub fn new(relative_path: NoteRelativePath, title: String, content: String) -> Self {
        Self {
            relative_path,
            title,
            content,
        }
    }
}
