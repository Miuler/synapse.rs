use std::path::PathBuf;

pub struct MergeStateQuery {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
}

pub struct ConflictVersionsQuery {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    /// Archivo en conflicto, relativo a la raíz del repositorio.
    pub path: String,
}
