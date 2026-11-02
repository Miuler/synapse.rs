use std::path::PathBuf;

pub struct PatchNoteCommand {
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub old_text: String,
    pub new_text: String,
    /// Línea aproximada para desambiguar si el texto aparece varias veces.
    pub start_line: Option<usize>,
    pub expected_hash: Option<String>,
    pub dry_run: bool,
}

/// Contenido antes y después del parche; `new_hash` solo si se guardó.
pub struct PatchNoteOutcome {
    pub old_content: String,
    pub new_content: String,
    pub new_hash: Option<String>,
}
