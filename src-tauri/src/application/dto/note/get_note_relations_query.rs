use std::path::PathBuf;

pub struct GetNoteRelationsQuery {
    pub vault_path: PathBuf,
    pub relative_path: String,
}

pub struct Backlink {
    pub source_path: String,
    pub title: String,
    pub context_snippet: String,
}

pub struct Outlink {
    pub target: String,
    pub alias: Option<String>,
    pub is_embed: bool,
    pub is_resolved: bool,
}

/// Enlaces entrantes y salientes de una nota y las notas con etiquetas más parecidas.
pub struct NoteRelations {
    pub target: String,
    pub backlinks: Vec<Backlink>,
    pub outlinks: Vec<Outlink>,
    /// Ruta y similitud de Jaccard de etiquetas (las 5 mayores).
    pub neighbors: Vec<(String, f64)>,
}
