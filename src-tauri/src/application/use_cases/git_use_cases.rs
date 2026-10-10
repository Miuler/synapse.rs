use crate::domain::models::git::{GitBranchesResult, GitDiffResponse, VaultGitStatus};
use crate::domain::services::git_service::GitPort;
use std::path::Path;

/// Casos de uso de la aplicación para operaciones de Git.
pub struct GitUseCases<G: GitPort> {
    port: G,
}

impl<G: GitPort> GitUseCases<G> {
    pub fn new(port: G) -> Self {
        Self { port }
    }

    pub fn get_status(&self, vault_path: &Path) -> Result<VaultGitStatus, String> {
        self.port.get_vault_status(vault_path)
    }

    pub fn add_paths(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.port.git_add(vault_path, paths)
    }

    pub fn restore_paths(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.port.git_restore(vault_path, paths)
    }

    pub fn restore_staged_paths(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.port.git_restore_staged(vault_path, paths)
    }

    pub fn commit_paths(&self, vault_path: &Path, paths: &[String], message: &str) -> Result<String, String> {
        self.port.git_commit(vault_path, paths, message)
    }

    pub fn get_file_diff(&self, vault_path: &Path, path: &str, staged: bool) -> Result<GitDiffResponse, String> {
        self.port.git_diff(vault_path, path, staged)
    }

    pub fn get_branches(&self, vault_path: &Path) -> Result<GitBranchesResult, String> {
        self.port.get_branches(vault_path)
    }

    pub fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String> {
        self.port.checkout_branch(vault_path, branch_name)
    }

    pub fn create_branch(&self, vault_path: &Path, new_branch: &str, base_branch: &str, checkout: bool) -> Result<String, String> {
        self.port.create_branch(vault_path, new_branch, base_branch, checkout)
    }
}
