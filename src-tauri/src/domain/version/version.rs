use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FileVersionState {
    pub index: Option<String>,
    pub worktree: Option<String>,
    pub is_stashed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultVersionState {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub statuses: HashMap<String, FileVersionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub is_staged: bool,
    pub old_file_name: String,
    pub new_file_name: String,
    pub old_content: String,
    pub new_content: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub ref_name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub last_commit_date: Option<String>,
    pub last_commit_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchList {
    pub current_branch: Option<String>,
    pub local_branches: Vec<Branch>,
    pub remote_branches: Vec<Branch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileRevision {
    pub path: String,
    pub revision: String,
    pub content: Option<String>,
    pub data_url: Option<String>,
    pub is_binary: bool,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageDiff {
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
pub struct VersionEntry {
    pub commit_hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub timestamp: i64,
    pub message: String,
    /// Tags que apuntan a este commit (p. ej. `v1.0.0`).
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileVersionHistory {
    pub path: String,
    pub commits: Vec<VersionEntry>,
}
