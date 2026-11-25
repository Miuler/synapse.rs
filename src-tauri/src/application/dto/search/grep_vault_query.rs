use std::path::PathBuf;

/// Líneas de los archivos de texto de la bóveda que cumplen una regex (como `rg`).
pub struct GrepVaultQuery {
    pub vault_path: PathBuf,
    pub pattern: String,
    /// `pattern` es texto literal, no regex.
    pub literal: bool,
    pub case_sensitive: bool,
    /// Carpeta relativa donde buscar; vacío = toda la bóveda.
    pub folder: String,
    /// Glob sobre la ruta (`*.sql`, `src/**/*.mmd`) para acotar los archivos.
    pub glob: Option<String>,
    /// Extensiones en minúsculas y sin punto; vacío = todas.
    pub extensions: Vec<String>,
    pub context_before: usize,
    pub context_after: usize,
    /// Solo los archivos con coincidencias y cuántas tienen (como `rg -c`), sin las líneas.
    pub files_only: bool,
    /// Máximo de líneas coincidentes devueltas en total.
    pub max_matches: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GrepLine {
    pub line: usize,
    pub text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GrepFile {
    pub path: String,
    pub match_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub matches: Vec<GrepLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrepVaultResult {
    pub files_searched: usize,
    /// Archivos omitidos por binarios o demasiado grandes.
    pub files_skipped: usize,
    pub total_matches: usize,
    /// Se dejaron de devolver líneas al llegar a `max_matches` (los recuentos siguen completos).
    pub truncated: bool,
    pub files: Vec<GrepFile>,
}
