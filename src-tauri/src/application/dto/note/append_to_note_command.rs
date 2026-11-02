use std::path::PathBuf;

pub struct AppendToNoteCommand {
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub content: String,
    /// Sección donde anexar; sin ella, al final de la nota.
    pub section_heading: Option<String>,
    /// `start_of_section` o `end_of_section`.
    pub position: String,
}
