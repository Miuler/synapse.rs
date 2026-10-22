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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitFileRevisionResult {
    pub path: String,
    pub revision: String,
    pub content: Option<String>,
    pub data_url: Option<String>,
    pub is_binary: bool,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitImageDiffResult {
    pub path: String,
    pub is_staged: bool,
    pub diff_data_url: Option<String>,
    pub diff_count: usize,
    pub width: u32,
    pub height: u32,
    pub total_pixels: usize,
    pub diff_percentage: f64,
    pub has_differences: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GitCommitLogItem {
    pub commit_hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub timestamp: i64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitFileLogResult {
    pub path: String,
    pub commits: Vec<GitCommitLogItem>,
}
