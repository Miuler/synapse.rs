use std::path::PathBuf;

pub struct ListFolderContentsQuery {
    pub vault_path: PathBuf,
    /// Carpeta relativa ya limpia (sin `/` en los extremos); vacía para la raíz.
    pub folder: String,
    pub include_notes: bool,
    pub include_folders: bool,
    /// Recorre subcarpetas (solo notas Markdown).
    pub recursive: bool,
    pub exclude_note_path: Option<String>,
}

pub struct FolderEntryInfo {
    pub name: String,
    pub relative_path: String,
}

pub struct FolderNoteInfo {
    pub name: String,
    pub title: String,
    pub relative_path: String,
    /// Solo en el listado no recursivo.
    pub line_count: Option<usize>,
}

pub struct FolderListing {
    pub folders: Vec<FolderEntryInfo>,
    pub notes: Vec<FolderNoteInfo>,
}
