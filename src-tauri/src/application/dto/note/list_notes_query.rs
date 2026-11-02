use std::path::PathBuf;

pub struct ListNotesQuery {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    pub extensions: Vec<String>,
}
