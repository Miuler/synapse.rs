use crate::domain::note::note_content::DiagramBlock;
use std::path::PathBuf;

pub struct ExtractNoteDiagramsQuery {
    pub vault_path: PathBuf,
    pub relative_path: String,
    /// `all`, `mermaid` o `d2`.
    pub diagram_type: String,
}

pub enum NoteDiagrams {
    /// Archivo `.d2` o `.mmd`: todo su contenido es un diagrama.
    DedicatedFile { kind: String, content: String },
    /// Bloques de diagrama dentro de una nota Markdown.
    Blocks(Vec<DiagramBlock>),
}
