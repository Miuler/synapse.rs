use crate::domain::version::version::{
    BranchList, CommitDetails, FileDiff, FileRevision, FileVersionHistory, ImageDiff, RepoLog, VaultVersionState,
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
    /// Log de todas las ramas y tags del repositorio de `vault_path`.
    fn get_repo_log(&self, vault_path: &Path, max_count: Option<usize>) -> Result<RepoLog, String>;
    fn get_commit_details(&self, vault_path: &Path, commit: &str) -> Result<CommitDetails, String>;
    fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String>;
    /// Crea un tag en `commit` (anotado si hay `message`).
    fn create_tag(&self, vault_path: &Path, name: &str, commit: &str, message: Option<&str>) -> Result<String, String>;
    fn delete_tag(&self, vault_path: &Path, name: &str) -> Result<String, String>;
    /// Borra una rama local; sin `force` git se niega si no está fusionada.
    fn delete_branch(&self, vault_path: &Path, name: &str, force: bool) -> Result<String, String>;
    /// Avanza la rama local `branch` hasta `target` solo si es un avance rápido (fast-forward).
    /// Con `dry_run` solo comprueba si se puede.
    fn fast_forward_branch(&self, vault_path: &Path, branch: &str, target: &str, dry_run: bool) -> Result<String, String>;
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

    fn get_repo_log(&self, vault_path: &Path, max_count: Option<usize>) -> Result<RepoLog, String> {
        (**self).get_repo_log(vault_path, max_count)
    }

    fn get_commit_details(&self, vault_path: &Path, commit: &str) -> Result<CommitDetails, String> {
        (**self).get_commit_details(vault_path, commit)
    }

    fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String> {
        (**self).checkout_branch(vault_path, branch_name)
    }

    fn create_tag(&self, vault_path: &Path, name: &str, commit: &str, message: Option<&str>) -> Result<String, String> {
        (**self).create_tag(vault_path, name, commit, message)
    }

    fn delete_tag(&self, vault_path: &Path, name: &str) -> Result<String, String> {
        (**self).delete_tag(vault_path, name)
    }

    fn delete_branch(&self, vault_path: &Path, name: &str, force: bool) -> Result<String, String> {
        (**self).delete_branch(vault_path, name, force)
    }

    fn fast_forward_branch(&self, vault_path: &Path, branch: &str, target: &str, dry_run: bool) -> Result<String, String> {
        (**self).fast_forward_branch(vault_path, branch, target, dry_run)
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
