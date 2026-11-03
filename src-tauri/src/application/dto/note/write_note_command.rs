use std::path::PathBuf;

pub struct WriteNoteCommand {
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub content: String,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
    /// Huella esperada del contenido actual (control de concurrencia optimista).
    pub expected_hash: Option<String>,
    pub allow_large_overwrite: bool,
    pub dry_run: bool,
}

pub enum WriteNoteOutcome {
    /// Ensayo: contenido anterior (vacío si la nota no existía) y el que se escribiría.
    Preview {
        old_content: String,
        new_content: String,
    },
    Saved {
        new_hash: String,
    },
}
