use crate::domain::models::workspace_registry::{RecentWorkspaceEntry, WorkspaceRegistryConfig};

pub trait WorkspaceRegistryRepository: Send + Sync {
    fn load_registry(&self) -> Result<WorkspaceRegistryConfig, String>;
    fn save_registry(&self, config: &WorkspaceRegistryConfig) -> Result<(), String>;
    fn record_workspace_opened(
        &self,
        path: &str,
        notes_count: Option<usize>,
        git_branch: Option<String>,
        is_git_repo: Option<bool>,
    ) -> Result<RecentWorkspaceEntry, String>;
    fn toggle_pin(&self, path: &str) -> Result<Vec<RecentWorkspaceEntry>, String>;
    fn remove_workspace(&self, path: &str) -> Result<Vec<RecentWorkspaceEntry>, String>;
    fn list_workspaces(&self) -> Result<Vec<RecentWorkspaceEntry>, String>;
}
