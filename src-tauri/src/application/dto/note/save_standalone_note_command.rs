use std::path::PathBuf;

pub struct SaveStandaloneNoteCommand {
    pub path: PathBuf,
    pub title: String,
    pub content: String,
    pub encoding: String,
}
