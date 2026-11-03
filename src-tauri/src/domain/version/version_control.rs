use crate::domain::version::version::{
    BranchList, FileDiff, FileRevision, FileVersionHistory, ImageDiff, VaultVersionState,
};
use std::path::Path;

pub trait VersionControl: Send + Sync {
    fn get_vault_status(&self, vault_path: &Path) -> Result<VaultVersionState, String>;
    fn is_tracked(&self, vault_path: &Path, relative_path: &str) -> bool;
    fn track(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn revert(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn untrack(&self, vault_path: &Path, paths: &[String]) -> Result<(), String>;
    fn save_version(
        &self,
        vault_path: &Path,
        paths: &[String],
        message: &str,
    ) -> Result<String, String>;
    fn diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
        revision: Option<&str>,
    ) -> Result<FileDiff, String>;
    fn get_file_revision(
        &self,
        vault_path: &Path,
        path: &str,
        revision: &str,
    ) -> Result<FileRevision, String>;
    fn get_image_diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
        revision: Option<&str>,
    ) -> Result<ImageDiff, String>;
    fn get_file_log(
        &self,
        vault_path: &Path,
        path: &str,
        max_count: Option<usize>,
    ) -> Result<FileVersionHistory, String>;
    fn get_branches(&self, vault_path: &Path) -> Result<BranchList, String>;
    fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String>;
    fn create_branch(
        &self,
        vault_path: &Path,
        new_branch: &str,
        base_branch: &str,
        checkout: bool,
    ) -> Result<String, String>;
    fn clone_repository(&self, repo_url: &str, target_path: &Path) -> Result<(), String>;
    fn push(&self, vault_path: &Path) -> Result<String, String>;
}

/// Permite pasar el repositorio por referencia a un caso de uso sin moverlo ni clonarlo.
impl<T: VersionControl + ?Sized> VersionControl for &T {
    fn get_vault_status(&self, vault_path: &Path) -> Result<VaultVersionState, String> {
        (**self).get_vault_status(vault_path)
    }

    fn is_tracked(&self, vault_path: &Path, relative_path: &str) -> bool {
        (**self).is_tracked(vault_path, relative_path)
    }

    fn track(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        (**self).track(vault_path, paths)
    }

    fn revert(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        (**self).revert(vault_path, paths)
    }

    fn untrack(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        (**self).untrack(vault_path, paths)
    }

    fn save_version(
        &self,
        vault_path: &Path,
        paths: &[String],
        message: &str,
    ) -> Result<String, String> {
        (**self).save_version(vault_path, paths, message)
    }

    fn diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
        revision: Option<&str>,
    ) -> Result<FileDiff, String> {
        (**self).diff(vault_path, path, staged, revision)
    }

    fn get_file_revision(
        &self,
        vault_path: &Path,
        path: &str,
        revision: &str,
    ) -> Result<FileRevision, String> {
        (**self).get_file_revision(vault_path, path, revision)
    }

    fn get_image_diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
        revision: Option<&str>,
    ) -> Result<ImageDiff, String> {
        (**self).get_image_diff(vault_path, path, staged, revision)
    }

    fn get_file_log(
        &self,
        vault_path: &Path,
        path: &str,
        max_count: Option<usize>,
    ) -> Result<FileVersionHistory, String> {
        (**self).get_file_log(vault_path, path, max_count)
    }

    fn get_branches(&self, vault_path: &Path) -> Result<BranchList, String> {
        (**self).get_branches(vault_path)
    }

    fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String> {
        (**self).checkout_branch(vault_path, branch_name)
    }

    fn create_branch(
        &self,
        vault_path: &Path,
        new_branch: &str,
        base_branch: &str,
        checkout: bool,
    ) -> Result<String, String> {
        (**self).create_branch(vault_path, new_branch, base_branch, checkout)
    }

    fn clone_repository(&self, repo_url: &str, target_path: &Path) -> Result<(), String> {
        (**self).clone_repository(repo_url, target_path)
    }

    fn push(&self, vault_path: &Path) -> Result<String, String> {
        (**self).push(vault_path)
    }
}
