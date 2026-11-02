use crate::domain::search::full_text::FullTextHit;
use std::path::PathBuf;

pub struct SearchNotesFullTextQuery {
    pub vault_path: PathBuf,
    /// Texto ya recortado y no vacío.
    pub text: String,
    pub limit: usize,
    /// Carpeta en minúsculas, sin `/` en los extremos.
    pub folder: Option<String>,
    pub tags: Vec<String>,
}

pub struct FullTextNoteHit {
    pub hit: FullTextHit,
    pub total_lines: usize,
    pub estimated_tokens: usize,
}

pub struct FullTextNoteSearch {
    pub total: usize,
    pub hits: Vec<FullTextNoteHit>,
}
