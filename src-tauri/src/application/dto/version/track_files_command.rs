use std::path::PathBuf;

pub struct TrackFilesCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub paths: Vec<String>,
}
