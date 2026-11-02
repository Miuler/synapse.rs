use std::path::PathBuf;

pub struct UpdateNoteMetadataCommand {
    pub vault_path: PathBuf,
    pub relative_path: String,
    pub title: Option<String>,
    pub add_tags: Option<Vec<String>>,
    pub remove_tags: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
}
