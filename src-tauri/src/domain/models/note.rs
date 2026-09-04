use crate::domain::value_objects::note_path::NoteRelativePath;
use serde::{Deserialize, Serialize};

fn default_encoding() -> String {
    "---".to_string()
}

/// Entidad Note que representa una nota dentro del dominio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub relative_path: NoteRelativePath,
    pub title: String,
    pub content: String,
    #[serde(default = "default_encoding")]
    pub encoding: String,
}

impl Note {
    pub fn new(relative_path: NoteRelativePath, title: String, content: String) -> Self {
        Self {
            relative_path,
            title,
            content,
            encoding: "---".to_string(),
        }
    }

    pub fn with_encoding(
        relative_path: NoteRelativePath,
        title: String,
        content: String,
        encoding: String,
    ) -> Self {
        Self {
            relative_path,
            title,
            content,
            encoding,
        }
    }
}
