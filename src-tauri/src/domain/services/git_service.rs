use crate::domain::models::git::{GitBranchesResult, GitDiffResponse, VaultGitStatus};
use std::path::Path;

pub trait GitPort: Send + Sync {
    fn get_vault_status(&self, vault_path: &Path) -> Result<VaultGitStatus, String>;
    fn git_add(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn git_restore(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn git_restore_staged(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn git_commit(&self, vault_path: &Path, paths: &[String], message: &str) -> Result<String, String>;
    fn git_diff(&self, vault_path: &Path, path: &str, staged: bool) -> Result<GitDiffResponse, String>;
    fn get_branches(&self, vault_path: &Path) -> Result<GitBranchesResult, String>;
    fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String>;
    fn create_branch(&self, vault_path: &Path, new_branch: &str, base_branch: &str, checkout: bool) -> Result<String, String>;
}
