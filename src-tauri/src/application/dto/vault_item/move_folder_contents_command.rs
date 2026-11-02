use std::path::PathBuf;

pub struct MoveFolderContentsCommand {
    pub vault_path: PathBuf,
    /// Carpetas ya limpias (sin `/` en los extremos).
    pub source_folder: String,
    pub target_folder: String,
    /// Extensión en minúsculas, sin punto.
    pub extension: Option<String>,
    /// `skip`, `overwrite` o `rename`.
    pub conflict_strategy: String,
    pub dry_run: bool,
}

pub enum MoveFolderContentsResult {
    /// Ensayo: archivos que se moverían (relativos a la carpeta origen).
    Preview { files: Vec<String> },
    Done {
        /// (archivo relativo a la carpeta origen, nueva ruta en la bóveda).
        moved: Vec<(String, String)>,
        skipped: Vec<String>,
        errors: Vec<String>,
    },
}
