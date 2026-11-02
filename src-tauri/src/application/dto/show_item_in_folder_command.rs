use std::path::PathBuf;

pub struct ShowItemInFolderCommand {
    pub vault_path: PathBuf,
    pub relative_path: String,
}
