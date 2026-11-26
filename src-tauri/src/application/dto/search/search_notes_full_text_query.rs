use crate::domain::search::full_text::{FullTextHit, LineOptions};
use std::path::PathBuf;

pub struct SearchNotesFullTextQuery {
    pub vault_path: PathBuf,
    /// Texto ya recortado y no vacío.
    pub text: String,
    pub limit: usize,
    /// Carpeta en minúsculas, sin `/` en los extremos.
    pub folder: Option<String>,
    pub tags: Vec<String>,
    /// Extensiones permitidas en minúsculas, sin punto (`archimate`, `md`…); vacío = todas.
    pub extensions: Vec<String>,
    /// Devolver las líneas que coinciden en cada resultado.
    pub lines: Option<LineOptions>,
    /// Modo exacto: textos literales que el documento debe contener (vacío = búsqueda normal).
    pub exact: Vec<String>,
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
