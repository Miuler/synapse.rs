use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GitFileStatus {
    pub index: Option<String>,
    pub worktree: Option<String>,
    pub is_stashed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultGitStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub statuses: HashMap<String, GitFileStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffResponse {
    pub path: String,
    pub is_staged: bool,
    pub old_file_name: String,
    pub new_file_name: String,
    pub old_content: String,
    pub new_content: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitBranchItem {
    pub name: String,
    pub ref_name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub last_commit_date: Option<String>,
    pub last_commit_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranchesResult {
    pub current_branch: Option<String>,
    pub local_branches: Vec<GitBranchItem>,
    pub remote_branches: Vec<GitBranchItem>,
}
