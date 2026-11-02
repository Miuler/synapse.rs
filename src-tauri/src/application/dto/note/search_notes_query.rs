use std::path::PathBuf;

pub struct SearchNotesQuery {
    pub vault_path: PathBuf,
    /// Texto en minúsculas; vacío para no filtrar por texto.
    pub query: String,
    pub limit: usize,
    /// Todas deben coincidir (de forma jerárquica).
    pub tags: Vec<String>,
}

pub struct NoteSearchHit {
    pub title: String,
    pub relative_path: String,
    pub tags: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
    pub description: Option<String>,
    pub snippet: String,
    pub total_lines: usize,
    pub estimated_tokens: usize,
}
