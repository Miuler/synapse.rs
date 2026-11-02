use std::path::PathBuf;

pub struct RevertFilesCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub paths: Vec<String>,
}
