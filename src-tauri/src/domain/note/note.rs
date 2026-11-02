use crate::domain::note::frontmatter::NoteFrontmatter;
use crate::domain::note::note_path::NoteRelativePath;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
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
    /// Título de una nota: el `title` del frontmatter; si no, el primer encabezado H1; si no,
    /// el nombre del archivo sin extensión.
    pub fn title_from_content(content: &str, file_stem: &str) -> String {
        // Si contiene frontmatter YAML con propiedad title, priorizarla
        let (fm, body) = crate::domain::note::frontmatter::NoteFrontmatter::parse(content);
        if let Some(fm) = fm {
            if let Some(explicit_title) = fm.title {
                let trimmed = explicit_title.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    
        let parser = Parser::new(&body);
        let mut in_h1 = false;
        let mut title = String::new();
    
        for event in parser {
            match event {
                Event::Start(Tag::Heading {
                    level: HeadingLevel::H1,
                    ..
                }) => {
                    in_h1 = true;
                }
                Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                    break;
                }
                Event::Text(text) if in_h1 => {
                    title.push_str(&text);
                }
                Event::Code(text) if in_h1 => {
                    title.push_str(&text);
                }
                _ => {}
            }
        }
    
        let trimmed = title.trim();
        if !trimmed.is_empty() {
            trimmed.to_string()
        } else {
            file_stem.to_string()
        }
    }

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
