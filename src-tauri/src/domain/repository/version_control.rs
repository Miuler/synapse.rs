use crate::domain::model::version::{
    BranchList, FileDiff, FileVersionHistory, FileRevision, ImageDiff,
    VaultVersionState,
};
use std::path::Path;

pub trait VersionControl: Send + Sync {
    fn get_vault_status(&self, vault_path: &Path) -> Result<VaultVersionState, String>;
    fn track(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn revert(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn untrack(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn save_version(&self, vault_path: &Path, paths: &[String], message: &str) -> Result<String, String>;
    fn diff(&self, vault_path: &Path, path: &str, staged: bool, revision: Option<&str>) -> Result<FileDiff, String>;
    fn get_file_revision(&self, vault_path: &Path, path: &str, revision: &str) -> Result<FileRevision, String>;
    fn get_image_diff(&self, vault_path: &Path, path: &str, staged: bool, revision: Option<&str>) -> Result<ImageDiff, String>;
    fn get_file_log(&self, vault_path: &Path, path: &str, max_count: Option<usize>) -> Result<FileVersionHistory, String>;
    fn get_branches(&self, vault_path: &Path) -> Result<BranchList, String>;
    fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String>;
    fn create_branch(&self, vault_path: &Path, new_branch: &str, base_branch: &str, checkout: bool) -> Result<String, String>;
    fn clone_repository(&self, repo_url: &str, target_path: &Path) -> Result<(), String>;
    fn push(&self, vault_path: &Path) -> Result<String, String>;
}
