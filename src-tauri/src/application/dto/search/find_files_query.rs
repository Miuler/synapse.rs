use std::path::PathBuf;

/// Archivos de la bóveda cuya ruta cumple un glob o una regex (como `fd`, en cualquier extensión).
pub struct FindFilesQuery {
    pub vault_path: PathBuf,
    /// Glob (`*ddl*.sql`, `src/**/*.mmd`): sin `/` mira el nombre; con `/`, la ruta entera.
    pub glob: Option<String>,
    /// Regex buscada en la ruta relativa (`entity|model|\.sql`).
    pub regex: Option<String>,
    pub case_sensitive: bool,
    /// Carpeta relativa donde buscar; vacío = toda la bóveda.
    pub folder: String,
    /// Extensiones en minúsculas y sin punto; vacío = todas.
    pub extensions: Vec<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FoundFile {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FoundFiles {
    /// Todos los que cumplen el filtro (no solo los devueltos).
    pub total: usize,
    pub files: Vec<FoundFile>,
}
