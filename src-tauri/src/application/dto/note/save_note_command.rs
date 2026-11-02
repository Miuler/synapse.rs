use std::path::PathBuf;

pub struct SaveNoteCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub title: String,
    pub content: String,
    pub encoding: String,
}
