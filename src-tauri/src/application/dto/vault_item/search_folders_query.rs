use crate::domain::vault::folder::FolderMeta;
use std::path::PathBuf;

pub struct SearchFoldersQuery {
    pub vault_path: PathBuf,
    /// Texto en minúsculas; vacío para listar todas.
    pub query: String,
    pub limit: usize,
    pub offset: usize,
}

pub struct FolderSearchResult {
    /// Total de carpetas que coinciden, antes de paginar.
    pub total: usize,
    pub folders: Vec<FolderMeta>,
}
