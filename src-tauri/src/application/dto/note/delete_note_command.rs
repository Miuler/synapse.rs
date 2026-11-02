use std::path::PathBuf;

pub struct DeleteNoteCommand {
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub expected_hash: Option<String>,
}
