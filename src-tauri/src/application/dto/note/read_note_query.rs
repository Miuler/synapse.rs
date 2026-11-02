use std::path::PathBuf;

pub struct ReadNoteQuery {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub relative_path: String,
}
