use crate::domain::models::frontmatter::NoteFrontmatter;
use crate::domain::value_objects::note_path::NoteRelativePath;
use serde::{Deserialize, Serialize};

fn default_encoding() -> String {
    "---".to_string()
}

/// Entidad Note que representa una nota dentro del dominio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub relative_path: NoteRelativePath,
    pub abs_path: String,
    pub title: String,
    pub content: String,
    #[serde(default = "default_encoding")]
    pub encoding: String,
    #[serde(default)]
    pub view_mode: Option<String>,
    #[serde(default)]
    pub frontmatter: Option<NoteFrontmatter>,
}

impl Note {
    pub fn new(
        relative_path: NoteRelativePath,
        abs_path: String,
        title: String,
        content: String,
    ) -> Self {
        Self {
            relative_path,
            abs_path,
            title,
            content,
            encoding: "---".to_string(),
            view_mode: None,
            frontmatter: None,
        }
    }

    pub fn with_encoding(
        relative_path: NoteRelativePath,
        abs_path: String,
        title: String,
        content: String,
        encoding: String,
    ) -> Self {
        Self {
            relative_path,
            abs_path,
            title,
            content,
            encoding,
            view_mode: None,
            frontmatter: None,
        }
    }

    pub fn with_frontmatter(mut self, frontmatter: Option<NoteFrontmatter>) -> Self {
        self.frontmatter = frontmatter;
        self
    }
}
