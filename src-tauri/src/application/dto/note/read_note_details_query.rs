use crate::domain::note::note::Note;
use crate::domain::note::note_content::Heading;
use std::path::PathBuf;

pub struct ReadNoteDetailsQuery {
    pub vault_path: PathBuf,
    pub relative_path: String,
}

/// Nota con su huella, tamaño, índice de encabezados y diagramas detectados.
pub struct NoteDetails {
    pub note: Note,
    pub content_hash: String,
    pub total_lines: usize,
    pub estimated_tokens: usize,
    pub toc: Vec<Heading>,
    pub mermaid_count: usize,
    pub d2_count: usize,
}
