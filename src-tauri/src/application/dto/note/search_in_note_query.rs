use std::path::PathBuf;

pub struct SearchInNoteQuery {
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub query: String,
    /// Líneas de contexto a cada lado (máximo 10).
    pub context_lines: usize,
}

/// Línea que coincide, con su contexto numerado.
pub struct LineMatch {
    pub line_number: usize,
    pub matched_line: String,
    pub context: Vec<String>,
}
