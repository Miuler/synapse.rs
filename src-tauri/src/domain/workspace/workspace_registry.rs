use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentWorkspaceEntry {
    pub path: String,
    pub display_path: String,
    pub name: String,
    pub last_opened_ms: u64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub notes_count: usize,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub is_git_repo: bool,
    #[serde(default)]
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkspaceRegistryConfig {
    #[serde(default)]
    pub workspaces: Vec<RecentWorkspaceEntry>,
    #[serde(default)]
    pub last_active_workspace: Option<String>,
}
