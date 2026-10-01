use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GitFileStatus {
    Modified,
    Untracked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultGitStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub statuses: HashMap<String, GitFileStatus>,
}

pub struct GitService;

impl GitService {
    pub fn new() -> Self {
        Self
    }

    pub fn get_vault_status(&self, vault_path: &Path) -> Result<VaultGitStatus, String> {
        let repo = match gix::discover(vault_path) {
            Ok(r) => r,
            Err(_) => {
                return Ok(VaultGitStatus {
                    is_repo: false,
                    branch: None,
                    statuses: HashMap::new(),
                });
            }
        };

        let work_dir = match repo.workdir() {
            Some(w) => w.to_path_buf(),
            None => {
                return Ok(VaultGitStatus {
                    is_repo: false,
                    branch: None,
                    statuses: HashMap::new(),
                });
            }
        };

        let branch = repo.head_name().ok().flatten().map(|n| n.shorten().to_string());

        let platform = match repo.status(gix::progress::Discard) {
            Ok(p) => p,
            Err(e) => return Err(format!("Failed to obtain git status platform: {e}")),
        };

        let status_iter = match platform
            .untracked_files(gix::status::UntrackedFiles::Files)
            .into_iter(None)
        {
            Ok(it) => it,
            Err(e) => return Err(format!("Failed to create git status iterator: {e}")),
        };

        let mut statuses: HashMap<String, GitFileStatus> = HashMap::new();

        for item in status_iter {
            match item {
                Ok(gix::status::Item::IndexWorktree(worktree_item)) => match worktree_item {
                    gix::status::index_worktree::Item::Modification { rela_path, .. } => {
                        let rel_str = rela_path.to_string();
                        let abs_path = work_dir.join(&rel_str);
                        if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                            let key = v_rel.to_string_lossy().to_string();
                            if !key.is_empty() {
                                statuses.insert(key, GitFileStatus::Modified);
                            }
                        }
                    }
                    gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                        if entry.status == gix::dir::entry::Status::Untracked {
                            let rel_str = entry.rela_path.to_string();
                            let abs_path = work_dir.join(&rel_str);
                            if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                                let key = v_rel.to_string_lossy().to_string();
                                if !key.is_empty() {
                                    statuses.insert(key, GitFileStatus::Untracked);
                                }
                            }
                        }
                    }
                    gix::status::index_worktree::Item::Rewrite { dirwalk_entry, .. } => {
                        let rel_str = dirwalk_entry.rela_path.to_string();
                        let abs_path = work_dir.join(&rel_str);
                        if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                            let key = v_rel.to_string_lossy().to_string();
                            if !key.is_empty() {
                                statuses.insert(key, GitFileStatus::Modified);
                            }
                        }
                    }
                },
                Ok(gix::status::Item::TreeIndex(change)) => {
                    let rel_str = change.location().to_string();
                    let abs_path = work_dir.join(&rel_str);
                    if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                        let key = v_rel.to_string_lossy().to_string();
                        if !key.is_empty() {
                            statuses.insert(key, GitFileStatus::Modified);
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(VaultGitStatus {
            is_repo: true,
            branch,
            statuses,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_repo_status() {
        let git_service = GitService::new();
        let current_dir = std::env::current_dir().unwrap();
        let res = git_service.get_vault_status(&current_dir);
        assert!(res.is_ok());
        let status = res.unwrap();
        assert!(status.is_repo);
        assert!(status.branch.is_some());
        println!("Git branch: {:?}", status.branch);
        println!("Modified/untracked files count: {}", status.statuses.len());
    }
}
