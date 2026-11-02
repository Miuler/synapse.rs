use std::path::PathBuf;

pub struct OpenFileWithDefaultAppCommand {
    pub vault_path: PathBuf,
    pub relative_path: String,
}
