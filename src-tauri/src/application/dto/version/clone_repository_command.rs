use std::path::PathBuf;

pub struct CloneRepositoryCommand {
    pub repo_url: String,
    pub target_path: PathBuf,
}
