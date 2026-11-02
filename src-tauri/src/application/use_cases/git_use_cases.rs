use crate::domain::model::version::{
    BranchList, FileDiff, FileVersionHistory, FileRevision, ImageDiff,
    VaultVersionState,
};
use crate::domain::repository::version_control::VersionControl;
use std::path::Path;

/// Casos de uso de la aplicación para operaciones de Git.
#[derive(Clone)]
pub struct GitUseCases<G: VersionControl> {
    repository: G,
}

impl<G: VersionControl> GitUseCases<G> {
    pub fn new(repository: G) -> Self {
        Self { repository }
    }

    pub fn get_status(&self, vault_path: &Path) -> Result<VaultVersionState, String> {
        self.repository.get_vault_status(vault_path)
    }

    pub fn add_paths(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.repository.track(vault_path, paths)
    }

    pub fn restore_paths(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.repository.revert(vault_path, paths)
    }

    pub fn restore_staged_paths(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.repository.untrack(vault_path, paths)
    }

    pub fn commit_paths(&self, vault_path: &Path, paths: &[String], message: &str) -> Result<String, String> {
        self.repository.save_version(vault_path, paths, message)
    }

    pub fn get_file_diff(&self, vault_path: &Path, path: &str, staged: bool, revision: Option<&str>) -> Result<FileDiff, String> {
        self.repository.diff(vault_path, path, staged, revision)
    }

    pub fn get_file_revision(&self, vault_path: &Path, path: &str, revision: &str) -> Result<FileRevision, String> {
        self.repository.get_file_revision(vault_path, path, revision)
    }

    pub fn get_image_diff(&self, vault_path: &Path, path: &str, staged: bool, revision: Option<&str>) -> Result<ImageDiff, String> {
        self.repository.get_image_diff(vault_path, path, staged, revision)
    }

    pub fn get_file_log(&self, vault_path: &Path, path: &str, max_count: Option<usize>) -> Result<FileVersionHistory, String> {
        self.repository.get_file_log(vault_path, path, max_count)
    }

    pub fn get_branches(&self, vault_path: &Path) -> Result<BranchList, String> {
        self.repository.get_branches(vault_path)
    }

    pub fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String> {
        self.repository.checkout_branch(vault_path, branch_name)
    }

    pub fn create_branch(&self, vault_path: &Path, new_branch: &str, base_branch: &str, checkout: bool) -> Result<String, String> {
        self.repository.create_branch(vault_path, new_branch, base_branch, checkout)
    }

    pub fn clone_repository(&self, repo_url: &str, target_path: &Path) -> Result<(), String> {
        self.repository.clone_repository(repo_url, target_path)
    }

    pub fn git_push(&self, vault_path: &Path) -> Result<String, String> {
        self.repository.push(vault_path)
    }
}
