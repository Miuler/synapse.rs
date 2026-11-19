use std::path::PathBuf;

pub struct CreateTagCommand {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    pub name: String,
    pub commit: String,
    pub message: Option<String>,
}
