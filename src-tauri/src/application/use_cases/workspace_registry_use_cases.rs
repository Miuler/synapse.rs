use crate::domain::models::workspace_registry::{RecentWorkspaceEntry, WorkspaceRegistryConfig};
use crate::domain::repositories::workspace_registry_repository::WorkspaceRegistryRepository;

pub struct WorkspaceRegistryUseCases<R: WorkspaceRegistryRepository> {
    repository: R,
}

impl<R: WorkspaceRegistryRepository> WorkspaceRegistryUseCases<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn get_registry(&self) -> Result<WorkspaceRegistryConfig, String> {
        self.repository.load_registry()
    }

    pub fn list_workspaces(&self) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.repository.list_workspaces()
    }

    pub fn record_workspace_opened(
        &self,
        path: &str,
        notes_count: Option<usize>,
        git_branch: Option<String>,
        is_git_repo: Option<bool>,
    ) -> Result<RecentWorkspaceEntry, String> {
        self.repository
            .record_workspace_opened(path, notes_count, git_branch, is_git_repo)
    }

    pub fn toggle_pin(&self, path: &str) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.repository.toggle_pin(path)
    }

    pub fn remove_workspace(&self, path: &str) -> Result<Vec<RecentWorkspaceEntry>, String> {
        self.repository.remove_workspace(path)
    }
}
