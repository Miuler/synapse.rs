pub struct RecordWorkspaceOpenedCommand {
    pub path: String,
    pub notes_count: Option<usize>,
    pub git_branch: Option<String>,
    pub is_git_repo: Option<bool>,
}
