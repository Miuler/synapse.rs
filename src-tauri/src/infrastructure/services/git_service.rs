pub use crate::domain::models::git::{
    GitBranchItem, GitBranchesResult, GitDiffResponse, GitFileStatus, VaultGitStatus,
};
use crate::domain::services::git_service::GitPort;
use std::collections::HashMap;
use std::path::Path;

pub struct GitService;

impl GitPort for GitService {
    fn get_vault_status(&self, vault_path: &Path) -> Result<VaultGitStatus, String> {
        self.get_vault_status(vault_path)
    }

    fn git_add(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.git_add(vault_path, paths)
    }

    fn git_restore(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.git_restore(vault_path, paths)
    }

    fn git_restore_staged(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        self.git_restore_staged(vault_path, paths)
    }

    fn git_commit(&self, vault_path: &Path, paths: &[String], message: &str) -> Result<String, String> {
        self.git_commit(vault_path, paths, message)
    }

    fn git_diff(&self, vault_path: &Path, path: &str, staged: bool) -> Result<GitDiffResponse, String> {
        self.git_diff(vault_path, path, staged)
    }

    fn get_branches(&self, vault_path: &Path) -> Result<GitBranchesResult, String> {
        self.get_branches(vault_path)
    }

    fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String> {
        self.checkout_branch(vault_path, branch_name)
    }

    fn create_branch(
        &self,
        vault_path: &Path,
        new_branch: &str,
        base_branch: &str,
        checkout: bool,
    ) -> Result<String, String> {
        self.create_branch(vault_path, new_branch, base_branch, checkout)
    }
}

impl GitService {
    pub fn new() -> Self {
        Self
    }

    fn collect_stashed_files(
        &self,
        repo: &gix::Repository,
        vault_path: &Path,
        work_dir: &Path,
    ) -> std::collections::HashSet<String> {
        let mut stashed = std::collections::HashSet::new();

        let Ok(mut stash_ref) = repo.find_reference("refs/stash") else {
            return stashed;
        };

        let mut commit_ids = Vec::new();
        if let Ok(commit) = stash_ref.peel_to_commit() {
            commit_ids.push(commit.id);
        }

        let mut platform = stash_ref.log_iter();
        if let Ok(Some(iter)) = platform.all() {
            for line_res in iter {
                if let Ok(line) = line_res {
                    let oid = line.new_oid();
                    if !commit_ids.contains(&oid) {
                        commit_ids.push(oid);
                    }
                }
            }
        }

        for commit_id in commit_ids {
            let commit = match repo
                .find_object(commit_id)
                .ok()
                .and_then(|o| o.try_into_commit().ok())
            {
                Some(c) => c,
                None => continue,
            };

            let parents: Vec<_> = commit.parent_ids().collect();
            if parents.is_empty() {
                continue;
            }

            // Parent 0 is the commit on top of which the stash was created
            if let Some(parent0) = repo
                .find_object(parents[0])
                .ok()
                .and_then(|o| o.try_into_commit().ok())
            {
                if let (Ok(p_tree), Ok(s_tree)) = (parent0.tree(), commit.tree()) {
                    if let Ok(mut diff) = p_tree.changes() {
                        let _ = diff.for_each_to_obtain_tree(&s_tree, |change| {
                            let rel_str = change.location().to_string();
                            let abs_path = work_dir.join(&rel_str);
                            if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                                let key = v_rel.to_string_lossy().to_string();
                                if !key.is_empty() && vault_path.join(&key).exists() {
                                    stashed.insert(key);
                                }
                            }
                            Ok(std::ops::ControlFlow::Continue(()))
                        });
                    }
                }
            }

            // Parent 2 (if present) contains untracked files stashed with `git stash -u`
            if parents.len() > 2 {
                if let Some(parent2) = repo
                    .find_object(parents[2])
                    .ok()
                    .and_then(|o| o.try_into_commit().ok())
                {
                    if let Ok(u_tree) = parent2.tree() {
                        let empty_tree = repo.empty_tree();
                        if let Ok(mut diff) = empty_tree.changes() {
                            let _ = diff.for_each_to_obtain_tree(&u_tree, |change| {
                                let rel_str = change.location().to_string();
                                let abs_path = work_dir.join(&rel_str);
                                if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                                    let key = v_rel.to_string_lossy().to_string();
                                    if !key.is_empty() && vault_path.join(&key).exists() {
                                        stashed.insert(key);
                                    }
                                }
                                Ok(std::ops::ControlFlow::Continue(()))
                            });
                        }
                    }
                }
            }
        }

        stashed
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

        let branch = repo
            .head_name()
            .ok()
            .flatten()
            .map(|n| n.shorten().to_string());

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
                                let entry = statuses.entry(key).or_default();
                                entry.worktree = Some("M".to_string());
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
                                    let entry = statuses.entry(key).or_default();
                                    entry.worktree = Some("?".to_string());
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
                                let entry = statuses.entry(key).or_default();
                                entry.worktree = Some("M".to_string());
                            }
                        }
                    }
                },
                Ok(gix::status::Item::TreeIndex(change)) => {
                    let idx_code = match &change {
                        gix::diff::index::ChangeRef::Addition { .. } => "A",
                        gix::diff::index::ChangeRef::Modification { .. } => "M",
                        gix::diff::index::ChangeRef::Deletion { .. } => "D",
                        gix::diff::index::ChangeRef::Rewrite { .. } => "M",
                    };
                    let rel_str = change.location().to_string();
                    let abs_path = work_dir.join(&rel_str);
                    if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                        let key = v_rel.to_string_lossy().to_string();
                        if !key.is_empty() {
                            let entry = statuses.entry(key).or_default();
                            entry.index = Some(idx_code.to_string());
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        let stashed_files = self.collect_stashed_files(&repo, vault_path, &work_dir);
        for key in stashed_files {
            let entry = statuses.entry(key).or_default();
            entry.is_stashed = true;
        }

        Ok(VaultGitStatus {
            is_repo: true,
            branch,
            statuses,
        })
    }

    pub fn git_add(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }

        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(work_dir);
        cmd.arg("add");
        cmd.arg("--");
        for p in paths {
            let clean = p.replace('\\', "/");
            let clean = clean.trim_start_matches("./");
            if !clean.is_empty() {
                cmd.arg(clean);
            }
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Error al ejecutar git add: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git add falló: {}", stderr.trim()));
        }

        Ok(())
    }

    pub fn git_restore(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }

        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(work_dir);
        cmd.arg("restore");
        cmd.arg("--");
        for p in paths {
            let clean = p.replace('\\', "/");
            let clean = clean.trim_start_matches("./");
            if !clean.is_empty() {
                cmd.arg(clean);
            }
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Error al ejecutar git restore: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git restore falló: {}", stderr.trim()));
        }

        Ok(())
    }

    pub fn git_restore_staged(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }

        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(work_dir);
        cmd.arg("restore");
        cmd.arg("--staged");
        cmd.arg("--");
        for p in paths {
            let clean = p.replace('\\', "/");
            let clean = clean.trim_start_matches("./");
            if !clean.is_empty() {
                cmd.arg(clean);
            }
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Error al ejecutar git restore --staged: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git restore --staged falló: {}", stderr.trim()));
        }

        Ok(())
    }

    pub fn git_commit(
        &self,
        vault_path: &Path,
        paths: &[String],
        message: &str,
    ) -> Result<String, String> {
        if paths.is_empty() {
            return Err("No se especificaron archivos para commitear".to_string());
        }
        let trimmed_msg = message.trim();
        if trimmed_msg.is_empty() {
            return Err("El mensaje de commit no puede estar vacío".to_string());
        }

        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        // 1. Añadir/preparar los archivos seleccionados para el commit
        self.git_add(vault_path, paths)?;

        // 2. Ejecutar git commit con pathspec
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(work_dir);
        cmd.arg("commit");
        cmd.arg("-m");
        cmd.arg(trimmed_msg);
        cmd.arg("--");
        for p in paths {
            let clean = p.replace('\\', "/");
            let clean = clean.trim_start_matches("./");
            if !clean.is_empty() {
                cmd.arg(clean);
            }
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Error al ejecutar git commit: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let err = if !stderr.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            return Err(format!("git commit falló: {}", err));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }

    pub fn git_diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
    ) -> Result<GitDiffResponse, String> {
        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let clean_path = path.replace('\\', "/");
        let clean_path = clean_path.trim_start_matches("./");

        if clean_path.is_empty() {
            return Err("Ruta de archivo no especificada para diff".to_string());
        }

        let abs_path = work_dir.join(clean_path);

        let mut old_content = String::new();
        let mut new_content = String::new();
        let mut diff = String::new();

        if staged {
            // Diff de stage: HEAD vs Index (:path)
            let mut show_head_cmd = std::process::Command::new("git");
            show_head_cmd.current_dir(work_dir);
            show_head_cmd.args(["show", &format!("HEAD:{}", clean_path)]);
            if let Ok(out) = show_head_cmd.output() {
                if out.status.success() {
                    old_content = String::from_utf8_lossy(&out.stdout).to_string();
                }
            }

            let mut show_index_cmd = std::process::Command::new("git");
            show_index_cmd.current_dir(work_dir);
            show_index_cmd.args(["show", &format!(":{}", clean_path)]);
            if let Ok(out) = show_index_cmd.output() {
                if out.status.success() {
                    new_content = String::from_utf8_lossy(&out.stdout).to_string();
                }
            }

            let mut diff_cmd = std::process::Command::new("git");
            diff_cmd.current_dir(work_dir);
            diff_cmd.args(["diff", "--staged", "-u", "--", clean_path]);
            if let Ok(out) = diff_cmd.output() {
                diff = String::from_utf8_lossy(&out.stdout).to_string();
            }
        } else {
            // Diff normal de trabajo: Index vs Worktree
            let mut show_index_cmd = std::process::Command::new("git");
            show_index_cmd.current_dir(work_dir);
            show_index_cmd.args(["show", &format!(":{}", clean_path)]);
            if let Ok(out) = show_index_cmd.output() {
                if out.status.success() {
                    old_content = String::from_utf8_lossy(&out.stdout).to_string();
                } else {
                    let mut show_head_cmd = std::process::Command::new("git");
                    show_head_cmd.current_dir(work_dir);
                    show_head_cmd.args(["show", &format!("HEAD:{}", clean_path)]);
                    if let Ok(head_out) = show_head_cmd.output() {
                        if head_out.status.success() {
                            old_content = String::from_utf8_lossy(&head_out.stdout).to_string();
                        }
                    }
                }
            }

            if abs_path.is_file() {
                if let Ok(c) = std::fs::read_to_string(&abs_path) {
                    new_content = c;
                }
            }

            let mut diff_cmd = std::process::Command::new("git");
            diff_cmd.current_dir(work_dir);
            diff_cmd.args(["diff", "-u", "--", clean_path]);
            if let Ok(out) = diff_cmd.output() {
                diff = String::from_utf8_lossy(&out.stdout).to_string();
            }

            // Si diff está vacío contra el index, comprobar si hay diferencias contra HEAD
            if diff.is_empty() {
                let mut head_diff_cmd = std::process::Command::new("git");
                head_diff_cmd.current_dir(work_dir);
                head_diff_cmd.args(["diff", "HEAD", "-u", "--", clean_path]);
                if let Ok(out) = head_diff_cmd.output() {
                    let head_diff = String::from_utf8_lossy(&out.stdout).to_string();
                    if !head_diff.is_empty() {
                        diff = head_diff;
                    }
                }
            }

            // Si diff sigue vacío pero es un archivo nuevo sin seguimiento (untracked)
            if diff.is_empty() && old_content.is_empty() && !new_content.is_empty() {
                #[cfg(windows)]
                let null_target = "NUL";
                #[cfg(not(windows))]
                let null_target = "/dev/null";

                let mut no_idx_cmd = std::process::Command::new("git");
                no_idx_cmd.current_dir(work_dir);
                no_idx_cmd.args(["diff", "--no-index", "-u", "--", null_target, clean_path]);
                if let Ok(out) = no_idx_cmd.output() {
                    diff = String::from_utf8_lossy(&out.stdout).to_string();
                }
            }
        }

        Ok(GitDiffResponse {
            path: clean_path.to_string(),
            is_staged: staged,
            old_file_name: clean_path.to_string(),
            new_file_name: clean_path.to_string(),
            old_content,
            new_content,
            diff,
        })
    }

    pub fn get_branches(&self, vault_path: &Path) -> Result<GitBranchesResult, String> {
        let repo = gix::discover(vault_path).map_err(|e| format!("Not a git repository: {e}"))?;
        let work_dir = repo
            .workdir()
            .ok_or_else(|| "Bare repository not supported".to_string())?
            .to_path_buf();

        let current_branch = repo
            .head_name()
            .ok()
            .flatten()
            .map(|n| n.shorten().to_string());

        let output = std::process::Command::new("git")
            .current_dir(&work_dir)
            .args([
                "for-each-ref",
                "--format=%(refname)|%(refname:short)|%(HEAD)|%(upstream:short)|%(committerdate:relative)|%(subject)",
                "refs/heads",
                "refs/remotes",
            ])
            .output()
            .map_err(|e| format!("Failed to execute git for-each-ref: {e}"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git for-each-ref failed: {err}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut local_branches = Vec::new();
        let mut remote_branches = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(6, '|').collect();
            if parts.len() < 6 {
                continue;
            }

            let ref_name = parts[0].trim();
            let short_name = parts[1].trim();
            let is_head_marker = parts[2].trim() == "*";
            let upstream_raw = parts[3].trim();
            let committerdate_rel = parts[4].trim();
            let subject = parts[5].trim();

            // Filtrar referencias simbólicas remotas como refs/remotes/origin/HEAD
            if ref_name.starts_with("refs/remotes/") && ref_name.ends_with("/HEAD") {
                continue;
            }

            let is_remote = ref_name.starts_with("refs/remotes/");
            let is_current = is_head_marker
                || current_branch
                    .as_ref()
                    .map(|b| b == short_name)
                    .unwrap_or(false);

            let branch_item = GitBranchItem {
                name: short_name.to_string(),
                ref_name: ref_name.to_string(),
                is_current,
                is_remote,
                upstream: if upstream_raw.is_empty() {
                    None
                } else {
                    Some(upstream_raw.to_string())
                },
                last_commit_date: if committerdate_rel.is_empty() {
                    None
                } else {
                    Some(committerdate_rel.to_string())
                },
                last_commit_message: if subject.is_empty() {
                    None
                } else {
                    Some(subject.to_string())
                },
            };

            if is_remote {
                remote_branches.push(branch_item);
            } else {
                local_branches.push(branch_item);
            }
        }

        Ok(GitBranchesResult {
            current_branch,
            local_branches,
            remote_branches,
        })
    }

    pub fn checkout_branch(&self, vault_path: &Path, branch_name: &str) -> Result<String, String> {
        let repo = gix::discover(vault_path).map_err(|e| format!("Not a git repository: {e}"))?;
        let work_dir = repo
            .workdir()
            .ok_or_else(|| "Bare repository not supported".to_string())?
            .to_path_buf();

        let branch_name = branch_name.trim();
        if branch_name.is_empty() {
            return Err("Branch name cannot be empty".to_string());
        }

        // Si es una rama remota ("origin/foo" o "refs/remotes/origin/foo")
        let target_branch = if branch_name.starts_with("refs/remotes/") {
            branch_name.trim_start_matches("refs/remotes/")
        } else {
            branch_name
        };

        // Si empieza con un nombre de remoto (e.g., origin/foo), verificar si existe rama local con ese nombre corto
        let candidate_local = if let Some((_remote, local_candidate)) = target_branch.split_once('/') {
            // Verificar si la rama local existe
            let check_local = std::process::Command::new("git")
                .current_dir(&work_dir)
                .args(["rev-parse", "--verify", local_candidate])
                .output();

            if let Ok(out) = check_local {
                if out.status.success() {
                    Some(local_candidate)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let checkout_target = candidate_local.unwrap_or(target_branch);

        let output = std::process::Command::new("git")
            .current_dir(&work_dir)
            .env("LC_ALL", "C")
            .args(["checkout", checkout_target])
            .output()
            .map_err(|e| format!("Failed to execute git checkout: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("overwritten by checkout") || stderr.contains("would be overwritten") {
                return Err(format!(
                    "Conflicto: Tienes cambios locales sin guardar que serían sobreescritos al cambiar a '{checkout_target}'. Por favor haz commit o stash antes de cambiar de rama."
                ));
            }
            return Err(format!("Error al cambiar de rama: {}", stderr.trim()));
        }

        let msg = format!("Cambiado con éxito a la rama '{checkout_target}'");
        Ok(msg)
    }

    pub fn create_branch(
        &self,
        vault_path: &Path,
        new_branch: &str,
        base_branch: &str,
        checkout: bool,
    ) -> Result<String, String> {
        let repo = gix::discover(vault_path).map_err(|e| format!("Not a git repository: {e}"))?;
        let work_dir = repo
            .workdir()
            .ok_or_else(|| "Bare repository not supported".to_string())?
            .to_path_buf();

        let new_branch = new_branch.trim();
        let base_branch = base_branch.trim();

        if new_branch.is_empty() {
            return Err("El nombre de la nueva rama no puede estar vacío".to_string());
        }

        // Validar formato de nombre con check-ref-format
        let check_format = std::process::Command::new("git")
            .current_dir(&work_dir)
            .args(["check-ref-format", "--branch", new_branch])
            .output()
            .map_err(|e| format!("Error al verificar formato de rama: {e}"))?;

        if !check_format.status.success() {
            return Err(format!("El nombre de rama '{new_branch}' no es válido según las reglas de Git."));
        }

        // Verificar si la rama ya existe localmente
        let check_exists = std::process::Command::new("git")
            .current_dir(&work_dir)
            .args(["rev-parse", "--verify", &format!("refs/heads/{new_branch}")])
            .output();

        if let Ok(out) = check_exists {
            if out.status.success() {
                return Err(format!("La rama '{new_branch}' ya existe"));
            }
        }

        let output = if checkout {
            if base_branch.is_empty() {
                std::process::Command::new("git")
                    .current_dir(&work_dir)
                    .args(["checkout", "-b", new_branch])
                    .output()
            } else {
                std::process::Command::new("git")
                    .current_dir(&work_dir)
                    .args(["checkout", "-b", new_branch, base_branch])
                    .output()
            }
        } else {
            if base_branch.is_empty() {
                std::process::Command::new("git")
                    .current_dir(&work_dir)
                    .args(["branch", new_branch])
                    .output()
            } else {
                std::process::Command::new("git")
                    .current_dir(&work_dir)
                    .args(["branch", new_branch, base_branch])
                    .output()
            }
        }.map_err(|e| format!("Failed to create branch: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Error al crear rama: {}", stderr.trim()));
        }

        let msg = if checkout {
            format!("Rama '{new_branch}' creada y activada correctamente")
        } else {
            format!("Rama '{new_branch}' creada correctamente a partir de '{base_branch}'")
        };

        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_stash_status() {
        let temp_dir = std::env::temp_dir().join(format!(
            "synapse_git_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let run_cmd = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&temp_dir)
                .status()
                .unwrap();
            assert!(status.success());
        };

        run_cmd(&["init"]);
        run_cmd(&["config", "user.name", "Test User"]);
        run_cmd(&["config", "user.email", "test@example.com"]);

        let file_a = temp_dir.join("file_a.txt");
        let file_b = temp_dir.join("file_b.txt");
        let file_c = temp_dir.join("file_c.txt");

        std::fs::write(&file_a, "initial a").unwrap();
        std::fs::write(&file_b, "initial b").unwrap();
        run_cmd(&["add", "file_a.txt", "file_b.txt"]);
        run_cmd(&["commit", "-m", "Initial commit"]);

        // 1. Modify file_a and stash it
        std::fs::write(&file_a, "stash change a").unwrap();
        run_cmd(&["stash", "push", "-m", "stash 1"]);

        // Working copy:
        // file_a is clean (reverted to "initial a"), but in stash
        // file_b is modified (unstaged)
        // file_c is untracked
        std::fs::write(&file_b, "modified b").unwrap();
        std::fs::write(&file_c, "untracked c").unwrap();

        let git_service = GitService::new();
        let status = git_service.get_vault_status(&temp_dir).unwrap();

        let stat_a = status.statuses.get("file_a.txt").unwrap();
        assert!(stat_a.is_stashed);
        assert_eq!(stat_a.index, None);
        assert_eq!(stat_a.worktree, None);

        let stat_b = status.statuses.get("file_b.txt").unwrap();
        assert_eq!(stat_b.worktree.as_deref(), Some("M"));
        assert_eq!(stat_b.index, None);

        let stat_c = status.statuses.get("file_c.txt").unwrap();
        assert_eq!(stat_c.worktree.as_deref(), Some("?"));

        // 2. Stage file_b (now it is staged modified 'M')
        run_cmd(&["add", "file_b.txt"]);
        let status = git_service.get_vault_status(&temp_dir).unwrap();
        let stat_b = status.statuses.get("file_b.txt").unwrap();
        assert_eq!(stat_b.index.as_deref(), Some("M"));
        assert_eq!(stat_b.worktree, None);

        // 3. Further modify file_b in worktree (now it has staged 'M' AND worktree 'M' -> 'MM'!)
        std::fs::write(&file_b, "modified b again").unwrap();
        let status = git_service.get_vault_status(&temp_dir).unwrap();
        let stat_b = status.statuses.get("file_b.txt").unwrap();
        assert_eq!(stat_b.index.as_deref(), Some("M"));
        assert_eq!(stat_b.worktree.as_deref(), Some("M"));

        // 4. Stage a new file (staged added 'A') and then modify it ('AM')
        let file_new = temp_dir.join("file_new.txt");
        std::fs::write(&file_new, "new file content").unwrap();
        run_cmd(&["add", "file_new.txt"]);

        let status = git_service.get_vault_status(&temp_dir).unwrap();
        let stat_new = status.statuses.get("file_new.txt").unwrap();
        assert_eq!(stat_new.index.as_deref(), Some("A"));
        assert_eq!(stat_new.worktree, None);

        std::fs::write(&file_new, "new file content modified").unwrap();
        let status = git_service.get_vault_status(&temp_dir).unwrap();
        let stat_new = status.statuses.get("file_new.txt").unwrap();
        assert_eq!(stat_new.index.as_deref(), Some("A"));
        assert_eq!(stat_new.worktree.as_deref(), Some("M"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_commit() {
        let temp_dir = std::env::temp_dir().join(format!(
            "synapse_git_commit_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let run_cmd = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&temp_dir)
                .status()
                .unwrap();
            assert!(status.success());
        };

        run_cmd(&["init"]);
        run_cmd(&["config", "user.name", "Test User"]);
        run_cmd(&["config", "user.email", "test@example.com"]);

        let file_a = temp_dir.join("file_a.txt");
        let file_b = temp_dir.join("file_b.txt");
        std::fs::write(&file_a, "initial a").unwrap();
        std::fs::write(&file_b, "initial b").unwrap();

        let git_service = GitService::new();
        let res = git_service.git_commit(&temp_dir, &["file_a.txt".to_string()], "commit a");
        assert!(res.is_ok());

        // Verify status: file_a is committed, file_b is still untracked
        let status = git_service.get_vault_status(&temp_dir).unwrap();
        assert!(!status.statuses.contains_key("file_a.txt"));
        let stat_b = status.statuses.get("file_b.txt").unwrap();
        assert_eq!(stat_b.worktree.as_deref(), Some("?"));

        // Now modify file_a and commit file_b
        std::fs::write(&file_a, "modified a").unwrap();
        let res_b = git_service.git_commit(&temp_dir, &["file_b.txt".to_string()], "commit b");
        assert!(res_b.is_ok());

        let status2 = git_service.get_vault_status(&temp_dir).unwrap();
        assert_eq!(status2.statuses.get("file_a.txt").unwrap().worktree.as_deref(), Some("M"));
        assert!(!status2.statuses.contains_key("file_b.txt"));

        // Test empty message error
        let err = git_service.git_commit(&temp_dir, &["file_a.txt".to_string()], "   ");
        assert!(err.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_diff() {
        let temp_dir = std::env::temp_dir().join(format!(
            "synapse_git_diff_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let run_cmd = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&temp_dir)
                .status()
                .unwrap();
            assert!(status.success());
        };

        run_cmd(&["init"]);
        run_cmd(&["config", "user.name", "Test User"]);
        run_cmd(&["config", "user.email", "test@example.com"]);

        let file_a = temp_dir.join("file_a.txt");
        std::fs::write(&file_a, "line 1\nline 2\n").unwrap();
        run_cmd(&["add", "file_a.txt"]);
        run_cmd(&["commit", "-m", "init"]);

        // 1. Modificar file_a en worktree (diff normal)
        std::fs::write(&file_a, "line 1\nline 2 mod\nline 3\n").unwrap();
        let git_service = GitService::new();
        let diff_worktree = git_service.git_diff(&temp_dir, "file_a.txt", false).unwrap();
        assert!(!diff_worktree.is_staged);
        assert_eq!(diff_worktree.old_content, "line 1\nline 2\n");
        assert_eq!(diff_worktree.new_content, "line 1\nline 2 mod\nline 3\n");
        assert!(diff_worktree.diff.contains("+line 2 mod"));

        // 2. Diff de stage debe estar vacío antes de add
        let diff_staged_empty = git_service.git_diff(&temp_dir, "file_a.txt", true).unwrap();
        assert!(diff_staged_empty.diff.is_empty());

        // 3. Stage del cambio y comprobar diff staged
        run_cmd(&["add", "file_a.txt"]);
        let diff_staged = git_service.git_diff(&temp_dir, "file_a.txt", true).unwrap();
        assert!(diff_staged.is_staged);
        assert_eq!(diff_staged.old_content, "line 1\nline 2\n");
        assert_eq!(diff_staged.new_content, "line 1\nline 2 mod\nline 3\n");
        assert!(diff_staged.diff.contains("+line 2 mod"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_branches_and_checkout() {
        let temp_dir = std::env::temp_dir().join(format!(
            "synapse_git_branches_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let run_cmd = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&temp_dir)
                .status()
                .unwrap();
            assert!(status.success());
        };

        run_cmd(&["init"]);
        run_cmd(&["config", "user.name", "Test User"]);
        run_cmd(&["config", "user.email", "test@example.com"]);

        let file_a = temp_dir.join("file_a.txt");
        std::fs::write(&file_a, "initial content\n").unwrap();
        run_cmd(&["add", "file_a.txt"]);
        run_cmd(&["commit", "-m", "initial commit"]);

        let git_service = GitService::new();

        // 1. List branches - initially master or main
        let branches = git_service.get_branches(&temp_dir).unwrap();
        assert!(!branches.local_branches.is_empty());
        let current = branches.current_branch.unwrap();
        assert!(branches.local_branches.iter().any(|b| b.name == current && b.is_current));

        // 2. Create a new branch without checkout
        let res = git_service.create_branch(&temp_dir, "feature-1", &current, false);
        assert!(res.is_ok());

        let branches = git_service.get_branches(&temp_dir).unwrap();
        assert!(branches.local_branches.iter().any(|b| b.name == "feature-1" && !b.is_current));

        // 3. Checkout branch
        let checkout_res = git_service.checkout_branch(&temp_dir, "feature-1");
        assert!(checkout_res.is_ok());

        let branches = git_service.get_branches(&temp_dir).unwrap();
        assert_eq!(branches.current_branch.as_deref(), Some("feature-1"));

        // 4. Create another branch with checkout
        let res2 = git_service.create_branch(&temp_dir, "feature-2", "", true);
        assert!(res2.is_ok());

        let branches = git_service.get_branches(&temp_dir).unwrap();
        assert_eq!(branches.current_branch.as_deref(), Some("feature-2"));

        // 5. Test conflict detection on checkout
        // Modify file_a on feature-2 and commit
        std::fs::write(&file_a, "feature-2 content\n").unwrap();
        run_cmd(&["add", "file_a.txt"]);
        run_cmd(&["commit", "-m", "feature-2 commit"]);

        // Switch back to feature-1
        git_service.checkout_branch(&temp_dir, "feature-1").unwrap();

        // Make an uncommitted change to file_a that conflicts with feature-2
        std::fs::write(&file_a, "uncommitted local change\n").unwrap();

        // Try checking out feature-2
        let conflict_res = git_service.checkout_branch(&temp_dir, "feature-2");
        assert!(conflict_res.is_err());
        assert!(conflict_res.unwrap_err().contains("Conflicto"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
