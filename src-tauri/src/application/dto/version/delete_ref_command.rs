use std::path::PathBuf;

/// Qué referencia borrar.
pub enum RefToDelete {
    Tag(String),
    /// Rama local; `force` = `git branch -D`.
    Branch { name: String, force: bool },
}

pub struct DeleteRefCommand {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    pub target: RefToDelete,
}
