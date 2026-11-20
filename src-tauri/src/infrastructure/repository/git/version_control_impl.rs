pub use crate::domain::version::version::{
    Branch, BranchList, CommitDetails, CommitFileChange, CommitRef, FileDiff, FileRevision,
    FileVersionHistory, FileVersionState, ImageDiff, RepoCommit, RepoLog, VaultVersionState,
    VersionEntry,
};
use crate::domain::version::version::RepoInfo;
use crate::domain::version::version_control::VersionControl;
use super::repo_routing as routing;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Copy, Default)]
pub struct VersionControlImpl;

/// Cada ruta va al repositorio más cercano que la contiene (la bóveda puede tener repositorios
/// en subcarpetas); las rutas que se devuelven vuelven a ser relativas a la bóveda.
impl VersionControl for VersionControlImpl {
    fn is_tracked(&self, vault_path: &Path, relative_path: &str) -> bool {
        let r = routing::route(vault_path, relative_path);
        VersionControlImpl::is_tracked(self, &r.root(vault_path), &r.inner)
    }

    fn get_vault_status(&self, vault_path: &Path) -> Result<VaultVersionState, String> {
        self.get_multi_repo_status(vault_path)
    }

    fn track(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        for (prefix, inner) in routing::group(vault_path, paths) {
            self.git_add(&repo_root(vault_path, &prefix), &inner)?;
        }
        Ok(())
    }

    fn revert(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        for (prefix, inner) in routing::group(vault_path, paths) {
            self.git_restore(&repo_root(vault_path, &prefix), &inner)?;
        }
        Ok(())
    }

    fn untrack(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        for (prefix, inner) in routing::group(vault_path, paths) {
            self.git_restore_staged(&repo_root(vault_path, &prefix), &inner)?;
        }
        Ok(())
    }

    fn save_version(
        &self,
        vault_path: &Path,
        paths: &[String],
        message: &str,
    ) -> Result<String, String> {
        // Un commit por repositorio: git no puede confirmar en varios a la vez.
        let mut out = Vec::new();
        for (prefix, inner) in routing::group(vault_path, paths) {
            let res = self.git_commit(&repo_root(vault_path, &prefix), &inner, message)?;
            out.push(if prefix.is_empty() { res } else { format!("[{prefix}] {res}") });
        }
        Ok(out.join("\n"))
    }

    fn diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
        revision: Option<&str>,
    ) -> Result<FileDiff, String> {
        let r = routing::route(vault_path, path);
        let mut d = self.git_diff(&r.root(vault_path), &r.inner, staged, revision)?;
        d.path = routing::prefixed(&r.prefix, &d.path);
        Ok(d)
    }

    fn get_file_revision(
        &self,
        vault_path: &Path,
        path: &str,
        revision: &str,
    ) -> Result<FileRevision, String> {
        let r = routing::route(vault_path, path);
        let mut rev = self.get_file_revision(&r.root(vault_path), &r.inner, revision)?;
        rev.path = routing::prefixed(&r.prefix, &rev.path);
        Ok(rev)
    }

    fn get_image_diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
        revision: Option<&str>,
    ) -> Result<ImageDiff, String> {
        let r = routing::route(vault_path, path);
        let mut d = self.get_image_diff(&r.root(vault_path), &r.inner, staged, revision)?;
        d.path = routing::prefixed(&r.prefix, &d.path);
        Ok(d)
    }

    fn get_file_log(
        &self,
        vault_path: &Path,
        path: &str,
        max_count: Option<usize>,
    ) -> Result<FileVersionHistory, String> {
        let r = routing::route(vault_path, path);
        let mut h = self.get_file_log(&r.root(vault_path), &r.inner, max_count)?;
        h.path = routing::prefixed(&r.prefix, &h.path);
        for c in &mut h.commits {
            if let Some(p) = c.previous_path.take() {
                c.previous_path = Some(routing::prefixed(&r.prefix, &p));
            }
        }
        Ok(h)
    }

    // Ramas y push: `vault_path` ya es la carpeta del repositorio elegido.
    fn get_branches(&self, vault_path: &Path) -> Result<BranchList, String> {
        self.get_branches(vault_path)
    }

    fn get_repo_log(&self, vault_path: &Path, max_count: Option<usize>) -> Result<RepoLog, String> {
        repo_log::get_repo_log(vault_path, max_count)
    }

    fn get_commit_details(&self, vault_path: &Path, commit: &str) -> Result<CommitDetails, String> {
        repo_log::get_commit_details(vault_path, commit)
    }

    fn create_tag(&self, vault_path: &Path, name: &str, commit: &str, message: Option<&str>) -> Result<String, String> {
        repo_log::create_tag(vault_path, name, commit, message)
    }

    fn delete_tag(&self, vault_path: &Path, name: &str) -> Result<String, String> {
        repo_log::delete_tag(vault_path, name)
    }

    fn delete_branch(&self, vault_path: &Path, name: &str, force: bool) -> Result<String, String> {
        repo_log::delete_branch(vault_path, name, force)
    }

    fn fast_forward_branch(&self, vault_path: &Path, branch: &str, target: &str, dry_run: bool) -> Result<String, String> {
        repo_log::fast_forward_branch(vault_path, branch, target, dry_run)
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

    fn clone_repository(&self, repo_url: &str, target_path: &Path) -> Result<(), String> {
        self.clone_repository(repo_url, target_path)
    }

    fn push(&self, vault_path: &Path) -> Result<String, String> {
        self.git_push(vault_path)
    }
}

fn repo_root(vault: &Path, prefix: &str) -> std::path::PathBuf {
    if prefix.is_empty() {
        vault.to_path_buf()
    } else {
        vault.join(prefix)
    }
}

impl VersionControlImpl {
    /// Estado de todos los repositorios de la bóveda (raíz y subcarpetas), con las rutas
    /// relativas a la bóveda. Los repositorios se consultan en paralelo.
    pub fn get_multi_repo_status(&self, vault_path: &Path) -> Result<VaultVersionState, String> {
        use rayon::prelude::*;
        let nested = routing::nested_repos(vault_path);
        let mut prefixes = vec![String::new()];
        prefixes.extend(nested.iter().cloned());
        let results: Vec<(String, Result<VaultVersionState, String>)> = prefixes
            .into_par_iter()
            .map(|prefix| {
                let st = self.get_vault_status(&repo_root(vault_path, &prefix));
                (prefix, st)
            })
            .collect();

        let mut out = VaultVersionState::default();
        for (prefix, res) in results {
            let st = match res {
                Ok(st) => st,
                // La raíz puede no ser un repositorio; un fallo en una subcarpeta no tumba el resto.
                Err(e) if prefix.is_empty() => return Err(e),
                Err(e) => {
                    log::warn!("[git] estado de {prefix}: {e}");
                    continue;
                }
            };
            if !st.is_repo {
                continue;
            }
            // Lo que la raíz ve dentro de un repositorio anidado lo informa ese repositorio.
            let inside_nested = |key: &str| {
                prefix.is_empty()
                    && nested.iter().any(|n| key == n || key.starts_with(&format!("{n}/")))
            };
            let mut changes = 0;
            for (k, v) in st.statuses {
                let k = k.trim_end_matches('/').to_string();
                if inside_nested(&k) {
                    continue;
                }
                changes += 1;
                out.statuses.insert(routing::prefixed(&prefix, &k), v);
            }
            if prefix.is_empty() {
                out.branch = st.branch.clone();
            }
            out.is_repo = true;
            out.repos.push(RepoInfo { path: prefix, branch: st.branch, changes });
        }
        Ok(out)
    }
}

/// Ruta relativa con `/` (en Windows `to_string_lossy` da `\\` y el explorador usa `/`).
fn rel_key(rel: &Path) -> String {
    let key = rel.to_string_lossy();
    if std::path::MAIN_SEPARATOR == '\\' { key.replace('\\', "/") } else { key.into_owned() }
}

impl VersionControlImpl {
    pub fn new() -> Self {
        Self
    }

    pub fn clone_repository(&self, repo_url: &str, target_path: &Path) -> Result<(), String> {
        let trimmed_url = repo_url.trim();
        if trimmed_url.is_empty() {
            return Err("La URL del repositorio Git no puede estar vacía".to_string());
        }

        let output = crate::infrastructure::service::os::git_binary::git_command()
            .arg("clone")
            .arg(trimmed_url)
            .arg(target_path)
            .output()
            .map_err(|e| format!("Error al ejecutar git clone: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git clone falló: {}", stderr.trim()));
        }

        Ok(())
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

    pub fn get_vault_status(&self, vault_path: &Path) -> Result<VaultVersionState, String> {
        let repo = match gix::discover(vault_path) {
            Ok(r) => r,
            Err(e) => {
                // Lo normal es que la carpeta no sea un repositorio; si lo es y gix no lo abre, aquí se ve por qué
                if vault_path.join(".git").exists() {
                    log::warn!("[git] {} tiene .git pero no se pudo abrir: {e}", vault_path.display());
                }
                return Ok(VaultVersionState {
                    is_repo: false,
                    branch: None,
                    statuses: HashMap::new(),
                    repos: Vec::new(),
                });
            }
        };

        let work_dir = match repo.workdir() {
            Some(w) => w.to_path_buf(),
            None => {
                return Ok(VaultVersionState {
                    is_repo: false,
                    branch: None,
                    statuses: HashMap::new(),
                    repos: Vec::new(),
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
            Err(e) => {
                // Sin estado de archivos el repositorio sigue siendo un repositorio: se muestra la
                // rama y las acciones de Git, y el motivo queda en el log.
                log::warn!("[git] no se pudo leer el estado de {}: {e}", work_dir.display());
                return Ok(VaultVersionState { is_repo: true, branch, statuses: HashMap::new(), repos: Vec::new() });
            }
        };

        let status_iter = match platform
            .untracked_files(gix::status::UntrackedFiles::Files)
            .into_iter(None)
        {
            Ok(it) => it,
            Err(e) => {
                // Sin estado de archivos el repositorio sigue siendo un repositorio: se muestra la
                // rama y las acciones de Git, y el motivo queda en el log.
                log::warn!("[git] no se pudo leer el estado de {}: {e}", work_dir.display());
                return Ok(VaultVersionState { is_repo: true, branch, statuses: HashMap::new(), repos: Vec::new() });
            }
        };

        let mut statuses: HashMap<String, FileVersionState> = HashMap::new();

        for item in status_iter {
            match item {
                Ok(gix::status::Item::IndexWorktree(worktree_item)) => match worktree_item {
                    gix::status::index_worktree::Item::Modification { rela_path, .. } => {
                        let rel_str = rela_path.to_string();
                        let abs_path = work_dir.join(&rel_str);
                        if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                            let key = rel_key(v_rel);
                            if !key.is_empty() {
                                let entry = statuses.entry(key).or_default();
                                let code = if abs_path.symlink_metadata().is_ok() {
                                    "M"
                                } else {
                                    "D"
                                };
                                entry.worktree = Some(code.to_string());
                            }
                        }
                    }
                    gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                        if entry.status == gix::dir::entry::Status::Untracked {
                            let rel_str = entry.rela_path.to_string();
                            let abs_path = work_dir.join(&rel_str);
                            if let Ok(v_rel) = abs_path.strip_prefix(vault_path) {
                                let key = rel_key(v_rel);
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
                            let key = rel_key(v_rel);
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
                        let key = rel_key(v_rel);
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

        Ok(VaultVersionState {
            is_repo: true,
            branch,
            statuses,
            repos: Vec::new(),
        })
    }

    pub fn git_add(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }

        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let prefix_from_workdir = vault_path.strip_prefix(work_dir).ok().and_then(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            let s = s.trim_matches('/').to_string();
            if s.is_empty() { None } else { Some(s) }
        });

        let mut targets: Vec<String> = Vec::new();
        for p in paths {
            let clean = p.replace('\\', "/");
            let clean = clean.trim_start_matches("./");
            if !clean.is_empty() {
                targets.push(match &prefix_from_workdir {
                    Some(prefix) => format!("{}/{}", prefix, clean),
                    None => clean.to_string(),
                });
            }
        }

        // Lo que ya no está en disco se quita del índice: `git add` falla con una ruta borrada que
        // ya no está en el índice (p. ej. un borrado que ya estaba en stage).
        let (existing, missing): (Vec<String>, Vec<String>) =
            targets.into_iter().partition(|t| work_dir.join(t).exists());
        if !missing.is_empty() {
            let output = crate::infrastructure::service::os::git_binary::git_command()
                .current_dir(work_dir)
                .args(["rm", "--cached", "-r", "-q", "--ignore-unmatch", "--"])
                .args(&missing)
                .output()
                .map_err(|e| format!("Error al ejecutar git rm: {}", e))?;
            if !output.status.success() {
                return Err(format!(
                    "git rm falló: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
        }

        let mut cmd = crate::infrastructure::service::os::git_binary::git_command();
        cmd.current_dir(work_dir);
        cmd.arg("add");
        cmd.arg("--");
        let count = existing.len();
        cmd.args(&existing);

        if count == 0 {
            return Ok(());
        }

        let output = cmd.output().map_err(|e| {
            let err_msg = format!("Error al ejecutar git add: {}", e);
            log::error!("[VersionControlImpl] {}", err_msg);
            eprintln!("[VersionControlImpl] {}", err_msg);
            err_msg
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if !stderr.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            let err_msg = format!("git add falló: {}", detail);
            log::error!("[VersionControlImpl] {}", err_msg);
            eprintln!("[VersionControlImpl] {}", err_msg);
            return Err(err_msg);
        }

        Ok(())
    }

    /// Indica si la ruta (archivo o carpeta) tiene archivos versionados en el índice de Git.
    /// Devuelve `false` fuera de un repositorio o ante cualquier error.
    pub fn is_tracked(&self, vault_path: &Path, rel_path: &str) -> bool {
        let Ok(repo) = gix::discover(vault_path) else {
            return false;
        };
        let work_dir = repo.workdir().unwrap_or(vault_path);
        let prefix = vault_path
            .strip_prefix(work_dir)
            .ok()
            .map(|p| {
                p.to_string_lossy()
                    .replace('\\', "/")
                    .trim_matches('/')
                    .to_string()
            })
            .filter(|s| !s.is_empty());
        let clean = rel_path.replace('\\', "/");
        let clean = clean.trim_start_matches("./");
        let target = match prefix {
            Some(pre) => format!("{}/{}", pre, clean),
            None => clean.to_string(),
        };
        crate::infrastructure::service::os::git_binary::git_command()
            .current_dir(work_dir)
            .args(["ls-files", "--", &target])
            .output()
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false)
    }

    pub fn git_restore(&self, vault_path: &Path, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }

        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        // Si la bóveda está en un subdirectorio del repositorio git, prefijar las rutas
        let prefix_from_workdir = vault_path.strip_prefix(work_dir).ok().and_then(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            let s = s.trim_matches('/').to_string();
            if s.is_empty() { None } else { Some(s) }
        });

        let mut cmd = crate::infrastructure::service::os::git_binary::git_command();
        cmd.current_dir(work_dir);
        cmd.arg("restore");
        cmd.arg("--");
        let mut count = 0;
        for p in paths {
            let clean = p.replace('\\', "/");
            let clean = clean.trim_start_matches("./");
            if !clean.is_empty() {
                let target_path = match &prefix_from_workdir {
                    Some(prefix) => format!("{}/{}", prefix, clean),
                    None => clean.to_string(),
                };
                cmd.arg(target_path);
                count += 1;
            }
        }

        if count == 0 {
            return Ok(());
        }

        let output = cmd.output().map_err(|e| {
            let err_msg = format!("Error al ejecutar git restore: {}", e);
            log::error!("[VersionControlImpl] {}", err_msg);
            eprintln!("[VersionControlImpl] {}", err_msg);
            err_msg
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if !stderr.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            let err_msg = format!("git restore falló: {}", detail);
            log::error!("[VersionControlImpl] {}", err_msg);
            eprintln!("[VersionControlImpl] {}", err_msg);
            return Err(err_msg);
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

        let prefix_from_workdir = vault_path.strip_prefix(work_dir).ok().and_then(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            let s = s.trim_matches('/').to_string();
            if s.is_empty() { None } else { Some(s) }
        });

        let mut cmd = crate::infrastructure::service::os::git_binary::git_command();
        cmd.current_dir(work_dir);
        cmd.arg("restore");
        cmd.arg("--staged");
        cmd.arg("--");
        let mut count = 0;
        for p in paths {
            let clean = p.replace('\\', "/");
            let clean = clean.trim_start_matches("./");
            if !clean.is_empty() {
                let target_path = match &prefix_from_workdir {
                    Some(prefix) => format!("{}/{}", prefix, clean),
                    None => clean.to_string(),
                };
                cmd.arg(target_path);
                count += 1;
            }
        }

        if count == 0 {
            return Ok(());
        }

        let output = cmd.output().map_err(|e| {
            let err_msg = format!("Error al ejecutar git restore --staged: {}", e);
            log::error!("[VersionControlImpl] {}", err_msg);
            eprintln!("[VersionControlImpl] {}", err_msg);
            err_msg
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if !stderr.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            let err_msg = format!("git restore --staged falló: {}", detail);
            log::error!("[VersionControlImpl] {}", err_msg);
            eprintln!("[VersionControlImpl] {}", err_msg);
            return Err(err_msg);
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

        let prefix_from_workdir = vault_path.strip_prefix(work_dir).ok().and_then(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            let s = s.trim_matches('/').to_string();
            if s.is_empty() { None } else { Some(s) }
        });

        // 1. Añadir/preparar los archivos seleccionados para el commit
        self.git_add(vault_path, paths)?;

        // 2. Ejecutar git commit con pathspec
        let mut cmd = crate::infrastructure::service::os::git_binary::git_command();
        cmd.current_dir(work_dir);
        cmd.arg("commit");
        cmd.arg("-m");
        cmd.arg(trimmed_msg);
        cmd.arg("--");
        for p in paths {
            let clean = p.replace('\\', "/");
            let clean = clean.trim_start_matches("./");
            if !clean.is_empty() {
                let target_path = match &prefix_from_workdir {
                    Some(prefix) => format!("{}/{}", prefix, clean),
                    None => clean.to_string(),
                };
                cmd.arg(target_path);
            }
        }

        let output = cmd.output().map_err(|e| {
            let err_msg = format!("Error al ejecutar git commit: {}", e);
            log::error!("[VersionControlImpl] {}", err_msg);
            eprintln!("[VersionControlImpl] {}", err_msg);
            err_msg
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let err = if !stderr.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            let err_msg = format!("git commit falló: {}", err);
            log::error!("[VersionControlImpl] {}", err_msg);
            eprintln!("[VersionControlImpl] {}", err_msg);
            return Err(err_msg);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }

    pub fn git_diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
        revision: Option<&str>,
    ) -> Result<FileDiff, String> {
        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let prefix_from_workdir = vault_path.strip_prefix(work_dir).ok().and_then(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            let s = s.trim_matches('/').to_string();
            if s.is_empty() { None } else { Some(s) }
        });

        let path_obj = Path::new(path);
        let path_str = if let Ok(rel) = path_obj.strip_prefix(vault_path) {
            rel.to_string_lossy().to_string()
        } else if let Ok(rel) = path_obj.strip_prefix(work_dir) {
            rel.to_string_lossy().to_string()
        } else {
            path.to_string()
        };

        let clean_path = path_str.replace('\\', "/");
        let clean_path = clean_path.trim_start_matches("./");
        let clean_path = clean_path.trim_start_matches('/');

        if clean_path.is_empty() {
            return Err("Ruta de archivo no especificada para diff".to_string());
        }

        let repo_rel_path = match &prefix_from_workdir {
            Some(prefix) => format!("{}/{}", prefix, clean_path),
            None => clean_path.to_string(),
        };

        let abs_path = work_dir.join(&repo_rel_path);

        let mut old_content = String::new();
        let mut new_content = String::new();
        let mut diff = String::new();
        let mut only_staged = false;

        if let Some(rev) = revision {
            // Diff contra una revisión histórica específica: rev vs Worktree. Si el archivo se
            // renombró después de esa revisión, se compara con el nombre que tenía entonces.
            let rev_path = Self::path_at_revision(work_dir, rev, &repo_rel_path)
                .unwrap_or_else(|| repo_rel_path.clone());
            let renamed = rev_path != repo_rel_path;
            let mut show_cmd = crate::infrastructure::service::os::git_binary::git_command();
            show_cmd.current_dir(work_dir);
            show_cmd.args(["show", &format!("{}:{}", rev, rev_path)]);
            if let Ok(out) = show_cmd.output() {
                if out.status.success() {
                    let resolved = Self::resolve_git_lfs_bytes_if_needed(
                        work_dir,
                        Some(repo.path()),
                        &out.stdout,
                    );
                    old_content = String::from_utf8_lossy(&resolved).to_string();
                }
            }

            if abs_path.is_file() {
                if let Ok(c) = std::fs::read_to_string(&abs_path) {
                    new_content = c;
                }
            }

            if renamed {
                // `git diff rev -- ruta` no ve el archivo con su nombre viejo: se arma el diff a mano
                let unified = similar::TextDiff::from_lines(&old_content, &new_content)
                    .unified_diff()
                    .header(&format!("a/{rev_path}"), &format!("b/{repo_rel_path}"))
                    .to_string();
                if !unified.is_empty() {
                    diff = format!("diff --git a/{rev_path} b/{repo_rel_path}\n{unified}");
                }
            } else {
                let mut diff_cmd = crate::infrastructure::service::os::git_binary::git_command();
                diff_cmd.current_dir(work_dir);
                diff_cmd.args(["diff", rev, "-u", "--", &repo_rel_path]);
                if let Ok(out) = diff_cmd.output() {
                    diff = String::from_utf8_lossy(&out.stdout).to_string();
                }
            }

            // Si diff está vacío pero el archivo local es nuevo o no existe en esa revisión
            if diff.is_empty() && old_content.is_empty() && !new_content.is_empty() {
                #[cfg(windows)]
                let null_target = "NUL";
                #[cfg(not(windows))]
                let null_target = "/dev/null";

                let mut no_idx_cmd = crate::infrastructure::service::os::git_binary::git_command();
                no_idx_cmd.current_dir(work_dir);
                no_idx_cmd.args([
                    "diff",
                    "--no-index",
                    "-u",
                    "--",
                    null_target,
                    &repo_rel_path,
                ]);
                if let Ok(out) = no_idx_cmd.output() {
                    diff = String::from_utf8_lossy(&out.stdout).to_string();
                }
            }
        } else if staged {
            // Diff de stage: HEAD vs Index (:path)
            let mut show_head_cmd = crate::infrastructure::service::os::git_binary::git_command();
            show_head_cmd.current_dir(work_dir);
            show_head_cmd.args(["show", &format!("HEAD:{}", repo_rel_path)]);
            if let Ok(out) = show_head_cmd.output() {
                if out.status.success() {
                    let resolved = Self::resolve_git_lfs_bytes_if_needed(
                        work_dir,
                        Some(repo.path()),
                        &out.stdout,
                    );
                    old_content = String::from_utf8_lossy(&resolved).to_string();
                }
            }

            let mut show_index_cmd = crate::infrastructure::service::os::git_binary::git_command();
            show_index_cmd.current_dir(work_dir);
            show_index_cmd.args(["show", &format!(":{}", repo_rel_path)]);
            if let Ok(out) = show_index_cmd.output() {
                if out.status.success() {
                    let resolved = Self::resolve_git_lfs_bytes_if_needed(
                        work_dir,
                        Some(repo.path()),
                        &out.stdout,
                    );
                    new_content = String::from_utf8_lossy(&resolved).to_string();
                }
            }

            let mut diff_cmd = crate::infrastructure::service::os::git_binary::git_command();
            diff_cmd.current_dir(work_dir);
            diff_cmd.args(["diff", "--staged", "-u", "--", &repo_rel_path]);
            if let Ok(out) = diff_cmd.output() {
                diff = String::from_utf8_lossy(&out.stdout).to_string();
            }
        } else {
            // Diff normal de trabajo: Index vs Worktree
            let mut show_index_cmd = crate::infrastructure::service::os::git_binary::git_command();
            show_index_cmd.current_dir(work_dir);
            show_index_cmd.args(["show", &format!(":{}", repo_rel_path)]);
            if let Ok(out) = show_index_cmd.output() {
                if out.status.success() {
                    let resolved = Self::resolve_git_lfs_bytes_if_needed(
                        work_dir,
                        Some(repo.path()),
                        &out.stdout,
                    );
                    old_content = String::from_utf8_lossy(&resolved).to_string();
                } else {
                    let mut show_head_cmd = crate::infrastructure::service::os::git_binary::git_command();
                    show_head_cmd.current_dir(work_dir);
                    show_head_cmd.args(["show", &format!("HEAD:{}", repo_rel_path)]);
                    if let Ok(head_out) = show_head_cmd.output() {
                        if head_out.status.success() {
                            let resolved = Self::resolve_git_lfs_bytes_if_needed(
                                work_dir,
                                Some(repo.path()),
                                &head_out.stdout,
                            );
                            old_content = String::from_utf8_lossy(&resolved).to_string();
                        }
                    }
                }
            }

            if abs_path.is_file() {
                if let Ok(c) = std::fs::read_to_string(&abs_path) {
                    new_content = c;
                }
            }

            let mut diff_cmd = crate::infrastructure::service::os::git_binary::git_command();
            diff_cmd.current_dir(work_dir);
            diff_cmd.args(["diff", "-u", "--", &repo_rel_path]);
            if let Ok(out) = diff_cmd.output() {
                diff = String::from_utf8_lossy(&out.stdout).to_string();
            }

            // Como `git diff`: local contra stage. Si no hay nada fuera de stage pero sí en stage, no
            // se cae a HEAD en silencio: se marca para que el visor ofrezca el diff de stage.
            if diff.is_empty() {
                let mut staged_cmd = crate::infrastructure::service::os::git_binary::git_command();
                staged_cmd.current_dir(work_dir);
                staged_cmd.args(["diff", "--staged", "--quiet", "--", &repo_rel_path]);
                if let Ok(out) = staged_cmd.output() {
                    only_staged = out.status.code() == Some(1);
                }
            }

            // Si diff sigue vacío pero es un archivo nuevo sin seguimiento (untracked)
            if diff.is_empty() && old_content.is_empty() && !new_content.is_empty() {
                #[cfg(windows)]
                let null_target = "NUL";
                #[cfg(not(windows))]
                let null_target = "/dev/null";

                let mut no_idx_cmd = crate::infrastructure::service::os::git_binary::git_command();
                no_idx_cmd.current_dir(work_dir);
                no_idx_cmd.args([
                    "diff",
                    "--no-index",
                    "-u",
                    "--",
                    null_target,
                    &repo_rel_path,
                ]);
                if let Ok(out) = no_idx_cmd.output() {
                    diff = String::from_utf8_lossy(&out.stdout).to_string();
                }
            }
        }

        Ok(FileDiff {
            path: clean_path.to_string(),
            is_staged: staged,
            old_file_name: clean_path.to_string(),
            new_file_name: clean_path.to_string(),
            old_content,
            new_content,
            diff,
            only_staged,
        })
    }

    fn resolve_git_lfs_bytes_if_needed(
        work_dir: &Path,
        git_dir: Option<&Path>,
        raw_bytes: &[u8],
    ) -> Vec<u8> {
        if !raw_bytes.starts_with(b"version https://git-lfs.github.com/spec/v1") {
            return raw_bytes.to_vec();
        }

        // 1. Intentar resolver usando `git lfs smudge`
        let mut cmd = crate::infrastructure::service::os::git_binary::git_command();
        cmd.current_dir(work_dir);
        cmd.args(["lfs", "smudge"]);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());

        if let Ok(mut child) = cmd.spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(raw_bytes);
            }
            if let Ok(output) = child.wait_with_output() {
                if output.status.success()
                    && !output.stdout.is_empty()
                    && !output
                        .stdout
                        .starts_with(b"version https://git-lfs.github.com/spec/v1")
                {
                    return output.stdout;
                }
            }
        }

        // 2. Fallback directo: buscar el blob en el almacenamiento local .git/lfs/objects/
        if let Ok(text) = std::str::from_utf8(raw_bytes) {
            for line in text.lines() {
                let line = line.trim();
                if let Some(oid) = line.strip_prefix("oid sha256:") {
                    let oid = oid.trim();
                    if oid.len() >= 4 {
                        let subpath = format!("lfs/objects/{}/{}/{}", &oid[0..2], &oid[2..4], oid);

                        if let Some(gd) = git_dir {
                            let candidate = gd.join(&subpath);
                            if candidate.is_file() {
                                if let Ok(bytes) = std::fs::read(&candidate) {
                                    return bytes;
                                }
                            }
                        }

                        let candidate = work_dir.join(".git").join(&subpath);
                        if candidate.is_file() {
                            if let Ok(bytes) = std::fs::read(&candidate) {
                                return bytes;
                            }
                        }
                    }
                }
            }
        }

        raw_bytes.to_vec()
    }

    /// Nombres que tuvo el archivo en su historial (siguiendo renombres), sin el actual.
    fn historical_names(work_dir: &Path, repo_rel_path: &str) -> Vec<String> {
        let out = crate::infrastructure::service::os::git_binary::git_command()
            .current_dir(work_dir)
            .args([
                "log",
                "--follow",
                "--name-status",
                "--format=",
                "--",
                repo_rel_path,
            ])
            .output();
        let Ok(out) = out else { return vec![] };
        let mut names: Vec<String> = Vec::new();
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            // «M\truta», «A\truta» o «R100\tvieja\tnueva»
            for name in line.split('\t').skip(1) {
                let name = name.trim();
                if !name.is_empty() && name != repo_rel_path && !names.iter().any(|n| n == name) {
                    names.push(name.to_string());
                }
            }
        }
        names
    }

    /// Ruta del archivo en `revision`: la actual si existe ahí, o el nombre que tenía entonces
    /// (si se renombró después). `None` si no existía en esa revisión con ningún nombre.
    pub fn path_at_revision(
        work_dir: &Path,
        revision: &str,
        repo_rel_path: &str,
    ) -> Option<String> {
        let exists = |p: &str| {
            crate::infrastructure::service::os::git_binary::git_command()
                .current_dir(work_dir)
                .args(["cat-file", "-e", &format!("{revision}:{p}")])
                .output()
                .is_ok_and(|o| o.status.success())
        };
        if exists(repo_rel_path) {
            return Some(repo_rel_path.to_string());
        }
        Self::historical_names(work_dir, repo_rel_path)
            .into_iter()
            .find(|name| exists(name))
    }

    pub fn get_file_revision(
        &self,
        vault_path: &Path,
        path: &str,
        revision: &str,
    ) -> Result<FileRevision, String> {
        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let prefix_from_workdir = vault_path.strip_prefix(work_dir).ok().and_then(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            let s = s.trim_matches('/').to_string();
            if s.is_empty() { None } else { Some(s) }
        });

        let path_obj = Path::new(path);
        let path_str = if let Ok(rel) = path_obj.strip_prefix(vault_path) {
            rel.to_string_lossy().to_string()
        } else if let Ok(rel) = path_obj.strip_prefix(work_dir) {
            rel.to_string_lossy().to_string()
        } else {
            path.to_string()
        };

        let clean_path = path_str.replace('\\', "/");
        let clean_path = clean_path.trim_start_matches("./");
        let clean_path = clean_path.trim_start_matches('/');

        if clean_path.is_empty() {
            return Err("Ruta de archivo no especificada para revisión".to_string());
        }

        let repo_rel_path = match &prefix_from_workdir {
            Some(prefix) => format!("{}/{}", prefix, clean_path),
            None => clean_path.to_string(),
        };

        // Formato para git show: "HEAD:path" o ":path" (para stage/index)
        let git_target = if revision == "staged" || revision == ":staged" || revision == ":" {
            format!(":{}", repo_rel_path)
        } else if revision.ends_with(':') {
            format!("{}{}", revision, repo_rel_path)
        } else if revision.is_empty() || revision.eq_ignore_ascii_case("head") {
            format!("HEAD:{}", repo_rel_path)
        } else {
            format!("{}:{}", revision, repo_rel_path)
        };

        let mut cmd = crate::infrastructure::service::os::git_binary::git_command();
        cmd.current_dir(work_dir);
        cmd.args(["show", &git_target]);

        let mut output = match cmd.output() {
            Ok(out) => out,
            Err(e) => return Err(format!("Error ejecutando git show: {}", e)),
        };

        // Ruta (relativa a la bóveda) con la que se leyó el archivo en esa revisión
        let mut shown_path = clean_path.to_string();
        // ¿Se renombró después? En esa revisión el archivo tenía otro nombre (como en `git log --follow`)
        let is_commit = !(revision == "staged"
            || revision == ":staged"
            || revision == ":"
            || revision.ends_with(':'));
        if !output.status.success() && is_commit {
            let rev = if revision.is_empty() {
                "HEAD"
            } else {
                revision
            };
            if let Some(old) = Self::path_at_revision(work_dir, rev, &repo_rel_path)
                .filter(|p| *p != repo_rel_path)
            {
                if let Ok(out) = crate::infrastructure::service::os::git_binary::git_command()
                    .current_dir(work_dir)
                    .args(["show", &format!("{rev}:{old}")])
                    .output()
                {
                    if out.status.success() {
                        shown_path = match &prefix_from_workdir {
                            Some(prefix) => old
                                .strip_prefix(&format!("{prefix}/"))
                                .unwrap_or(&old)
                                .to_string(),
                            None => old.clone(),
                        };
                    }
                    output = out;
                }
            }
        }

        if !output.status.success() {
            return Ok(FileRevision {
                path: clean_path.to_string(),
                revision: revision.to_string(),
                content: None,
                data_url: None,
                is_binary: false,
                exists: false,
            });
        }

        let raw_bytes =
            Self::resolve_git_lfs_bytes_if_needed(work_dir, Some(repo.path()), &output.stdout);
        let ext = Path::new(&clean_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let is_known_binary = matches!(
            ext.as_str(),
            "png"
                | "jpg"
                | "jpeg"
                | "webp"
                | "gif"
                | "bmp"
                | "ico"
                | "avif"
                | "tiff"
                | "tif"
                | "pdf"
        );

        // Intento de decodificación UTF-8
        let utf8_res = std::str::from_utf8(&raw_bytes);
        let is_binary = is_known_binary || utf8_res.is_err();

        let mime = match ext.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "bmp" => "image/bmp",
            "ico" => "image/x-icon",
            "avif" => "image/avif",
            "tiff" | "tif" => "image/tiff",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        };

        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
        let data_url = Some(format!(
            "data:{};base64,{}",
            mime,
            BASE64_STANDARD.encode(&raw_bytes)
        ));

        let content = if let Ok(s) = utf8_res {
            Some(s.to_string())
        } else {
            None
        };

        Ok(FileRevision {
            path: shown_path,
            revision: revision.to_string(),
            content,
            data_url,
            is_binary,
            exists: true,
        })
    }

    pub fn get_image_diff(
        &self,
        vault_path: &Path,
        path: &str,
        staged: bool,
        revision: Option<&str>,
    ) -> Result<ImageDiff, String> {
        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let prefix_from_workdir = vault_path.strip_prefix(work_dir).ok().and_then(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            let s = s.trim_matches('/').to_string();
            if s.is_empty() { None } else { Some(s) }
        });

        let path_obj = Path::new(path);
        let path_str = if let Ok(rel) = path_obj.strip_prefix(vault_path) {
            rel.to_string_lossy().to_string()
        } else if let Ok(rel) = path_obj.strip_prefix(work_dir) {
            rel.to_string_lossy().to_string()
        } else {
            path.to_string()
        };

        let clean_path = path_str.replace('\\', "/");
        let clean_path = clean_path.trim_start_matches("./");
        let clean_path = clean_path.trim_start_matches('/');

        if clean_path.is_empty() {
            return Err("Ruta de archivo no especificada para diff de imagen".to_string());
        }

        let repo_rel_path = match &prefix_from_workdir {
            Some(prefix) => format!("{}/{}", prefix, clean_path),
            None => clean_path.to_string(),
        };

        let abs_path = work_dir.join(&repo_rel_path);

        // 1. Obtener bytes de la imagen base / histórica (HEAD con fallback a index si no hay HEAD, o revisión explícita)
        let mut old_bytes = Vec::new();
        if let Some(rev) = revision {
            // Con el nombre que tenía el archivo en esa revisión (por si se renombró después)
            let rev_path = Self::path_at_revision(work_dir, rev, &repo_rel_path)
                .unwrap_or_else(|| repo_rel_path.clone());
            let mut rev_cmd = crate::infrastructure::service::os::git_binary::git_command();
            rev_cmd.current_dir(work_dir);
            rev_cmd.args(["show", &format!("{}:{}", rev, rev_path)]);
            if let Ok(out) = rev_cmd.output() {
                if out.status.success() {
                    old_bytes = out.stdout;
                }
            }
        } else {
            let mut head_cmd = crate::infrastructure::service::os::git_binary::git_command();
            head_cmd.current_dir(work_dir);
            head_cmd.args(["show", &format!("HEAD:{}", repo_rel_path)]);
            if let Ok(out) = head_cmd.output() {
                if out.status.success() {
                    old_bytes = out.stdout;
                }
            }

            if old_bytes.is_empty() && !staged {
                let mut index_cmd = crate::infrastructure::service::os::git_binary::git_command();
                index_cmd.current_dir(work_dir);
                index_cmd.args(["show", &format!(":{}", repo_rel_path)]);
                if let Ok(out) = index_cmd.output() {
                    if out.status.success() {
                        old_bytes = out.stdout;
                    }
                }
            }
        }

        // 2. Obtener bytes de la imagen actual (Working Tree o Staged Index)
        let new_bytes = if revision.is_some() {
            // Comparar revisión histórica contra disco duro local
            if abs_path.is_file() {
                std::fs::read(&abs_path).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else if staged {
            let mut show_index = crate::infrastructure::service::os::git_binary::git_command();
            show_index.current_dir(work_dir);
            show_index.args(["show", &format!(":{}", repo_rel_path)]);
            if let Ok(out) = show_index.output() {
                if out.status.success() {
                    out.stdout
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else if abs_path.is_file() {
            std::fs::read(&abs_path).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Resolver Git LFS si los bytes corresponden a un apuntador LFS
        let old_bytes =
            Self::resolve_git_lfs_bytes_if_needed(work_dir, Some(repo.path()), &old_bytes);
        let new_bytes =
            Self::resolve_git_lfs_bytes_if_needed(work_dir, Some(repo.path()), &new_bytes);

        if old_bytes.is_empty() && new_bytes.is_empty() {
            return Err("No se encontraron bytes de imagen para comparar".to_string());
        }

        if old_bytes.is_empty() || new_bytes.is_empty() {
            // Archivo agregado o borrado por completo
            return Ok(ImageDiff {
                path: clean_path.to_string(),
                is_staged: staged,
                diff_data_url: None,
                diff_count: 0,
                width: 0,
                height: 0,
                total_pixels: 0,
                diff_percentage: 100.0,
                has_differences: true,
            });
        }

        let ext = Path::new(&clean_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "svg" {
            return Err("El diff perceptual de píxeles no aplica a archivos SVG vectoriales (utiliza la vista dividida de 2 paneles o diff de texto)".to_string());
        }

        // 3. Decodificar ambas imágenes en memoria con la crate `image`
        let decode_image = |bytes: &[u8]| -> Result<image::RgbaImage, String> {
            let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes));
            reader = match reader.with_guessed_format() {
                Ok(r) => r,
                Err(_) => {
                    if let Some(fmt) = image::ImageFormat::from_extension(&ext) {
                        let mut r2 = image::ImageReader::new(std::io::Cursor::new(bytes));
                        r2.set_format(fmt);
                        r2
                    } else {
                        return Err(format!(
                            "Formato de imagen '{}' no soportado o desconocido",
                            ext
                        ));
                    }
                }
            };
            if reader.format().is_none() {
                if let Some(fmt) = image::ImageFormat::from_extension(&ext) {
                    reader.set_format(fmt);
                }
            }
            reader
                .decode()
                .map(|img| img.to_rgba8())
                .map_err(|e| format!("Error decodificando imagen: {}", e))
        };

        let img1 = decode_image(&old_bytes)
            .map_err(|e| format!("Error decodificando imagen base de Git: {}", e))?;

        let img2 = decode_image(&new_bytes)
            .map_err(|e| format!("Error decodificando imagen modificada: {}", e))?;

        // 4. Normalizar dimensiones si difieren
        let max_w = img1.width().max(img2.width());
        let max_h = img1.height().max(img2.height());

        let mut canvas1 = image::RgbaImage::new(max_w, max_h);
        image::imageops::overlay(&mut canvas1, &img1, 0, 0);

        let mut canvas2 = image::RgbaImage::new(max_w, max_h);
        image::imageops::overlay(&mut canvas2, &img2, 0, 0);

        // Codificar canvas1 y canvas2 en PNG en memoria para alimentar pixelmatch
        let mut png_buf1 = std::io::Cursor::new(Vec::new());
        canvas1
            .write_to(&mut png_buf1, image::ImageFormat::Png)
            .map_err(|e| format!("Error preparando buffer PNG 1: {}", e))?;

        let mut png_buf2 = std::io::Cursor::new(Vec::new());
        canvas2
            .write_to(&mut png_buf2, image::ImageFormat::Png)
            .map_err(|e| format!("Error preparando buffer PNG 2: {}", e))?;

        let mut out_png_buffer = std::io::Cursor::new(Vec::new());

        // 5. Comparar píxeles con `pixelmatch` directamente en memoria
        let diff_count = pixelmatch::pixelmatch(
            std::io::Cursor::new(png_buf1.into_inner()),
            std::io::Cursor::new(png_buf2.into_inner()),
            Some(&mut out_png_buffer),
            Some(max_w),
            Some(max_h),
            Some(pixelmatch::Options {
                threshold: 0.1,
                include_aa: true,
                alpha: 0.2,                     // Atenuar fondo no modificado al 20%
                aa_color: [255, 140, 0, 255],   // Naranja para diferencias de anti-aliasing
                diff_color: [255, 0, 119, 255], // Magenta brillante para diferencias reales
                diff_color_alt: None,
                diff_mask: false,
            }),
        )
        .map_err(|e| format!("Error ejecutando pixelmatch: {}", e))?;

        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
        let diff_data_url = format!(
            "data:image/png;base64,{}",
            BASE64_STANDARD.encode(out_png_buffer.into_inner())
        );

        let total_pixels = (max_w * max_h) as usize;
        let diff_percentage = if total_pixels > 0 {
            (diff_count as f64 / total_pixels as f64) * 100.0
        } else {
            0.0
        };

        Ok(ImageDiff {
            path: clean_path.to_string(),
            is_staged: staged,
            diff_data_url: Some(diff_data_url),
            diff_count,
            width: max_w,
            height: max_h,
            total_pixels,
            diff_percentage,
            has_differences: diff_count > 0 || img1.dimensions() != img2.dimensions(),
        })
    }

    pub fn get_file_log(
        &self,
        vault_path: &Path,
        path: &str,
        max_count: Option<usize>,
    ) -> Result<FileVersionHistory, String> {
        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {}", e))?;
        let work_dir = repo.workdir().unwrap_or(vault_path);

        let prefix_from_workdir = vault_path.strip_prefix(work_dir).ok().and_then(|p| {
            let s = p.to_string_lossy().replace('\\', "/");
            let s = s.trim_matches('/').to_string();
            if s.is_empty() { None } else { Some(s) }
        });

        let path_obj = Path::new(path);
        let path_str = if let Ok(rel) = path_obj.strip_prefix(vault_path) {
            rel.to_string_lossy().to_string()
        } else if let Ok(rel) = path_obj.strip_prefix(work_dir) {
            rel.to_string_lossy().to_string()
        } else {
            path.to_string()
        };

        let clean_path = path_str.replace('\\', "/");
        let clean_path = clean_path.trim_start_matches("./");
        let clean_path = clean_path.trim_start_matches('/');

        if clean_path.is_empty() {
            return Err("Ruta de archivo no especificada para historial".to_string());
        }

        let repo_rel_path = match &prefix_from_workdir {
            Some(prefix) => format!("{}/{}", prefix, clean_path),
            None => clean_path.to_string(),
        };

        let limit = max_count.unwrap_or(100).to_string();

        // Historial con el grafo: sin `--follow` (que no reescribe los padres) pero con todos los
        // nombres que tuvo el archivo; `--full-history --simplify-merges` conserva las ramas que
        // también lo tocaron y las fusiones que las unen.
        let mut paths = vec![repo_rel_path.clone()];
        paths.extend(Self::historical_names(work_dir, &repo_rel_path));
        let mut cmd = crate::infrastructure::service::os::git_binary::git_command();
        cmd.current_dir(work_dir);
        // \x1e separa commits; tras el formato vienen las líneas de --name-status del commit.
        // %H %h %an %ae %aI %at, %D: tags (por --decorate-refs), %P: padres reescritos, %s: asunto
        cmd.args([
            "log",
            "--full-history",
            "--simplify-merges",
            "--parents",
            "--name-status",
            "--find-renames",
            "--decorate-refs=refs/tags/",
            "--format=\x1e%H\x1f%h\x1f%an\x1f%ae\x1f%aI\x1f%at\x1f%D\x1f%P\x1f%s",
            "-n",
            &limit,
            "--",
        ]);
        cmd.args(&paths);

        let output = cmd
            .output()
            .map_err(|e| format!("Error ejecutando git log: {}", e))?;

        let mut commits = Vec::new();
        if output.status.success() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            for block in stdout_str.split('\x1e').filter(|b| !b.trim().is_empty()) {
                let mut lines = block.lines();
                let Some(header) = lines.next() else { continue };
                let parts: Vec<&str> = header.split('\x1f').collect();
                if parts.len() < 9 {
                    continue;
                }
                // Cambio sobre alguno de los nombres del archivo («M\truta», «R090\tvieja\tnueva»)
                let mut change = None;
                let mut previous_path = None;
                for line in lines {
                    let cols: Vec<&str> = line.split('\t').collect();
                    if cols.len() < 2 || !cols[1..].iter().any(|c| paths.iter().any(|p| p == c)) {
                        continue;
                    }
                    let kind = &cols[0][..1];
                    if kind == "R" && cols.len() >= 3 {
                        let strip = |p: &str| match &prefix_from_workdir {
                            Some(prefix) => p.strip_prefix(&format!("{prefix}/")).unwrap_or(p).to_string(),
                            None => p.to_string(),
                        };
                        previous_path = Some(strip(cols[1]));
                    }
                    change = Some(kind.to_string());
                    break;
                }
                commits.push(VersionEntry {
                    commit_hash: parts[0].trim().to_string(),
                    short_hash: parts[1].trim().to_string(),
                    author_name: parts[2].trim().to_string(),
                    author_email: parts[3].trim().to_string(),
                    date: parts[4].trim().to_string(),
                    timestamp: parts[5].trim().parse::<i64>().unwrap_or(0),
                    tags: parse_tag_refs(parts[6]),
                    parents: parts[7].split_whitespace().map(str::to_string).collect(),
                    message: parts[8..].join("\x1f").trim().to_string(),
                    change,
                    previous_path,
                });
            }
        }

        Ok(FileVersionHistory {
            path: clean_path.to_string(),
            commits,
        })
    }

    pub fn get_branches(&self, vault_path: &Path) -> Result<BranchList, String> {
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

        let output = crate::infrastructure::service::os::git_binary::git_command()
            .current_dir(&work_dir)
            .args([
                "for-each-ref",
                "--format=%(refname)|%(refname:short)|%(HEAD)|%(upstream:short)|%(upstream:track,nobracket)|%(committerdate:relative)|%(subject)",
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

            let parts: Vec<&str> = line.splitn(7, '|').collect();
            if parts.len() < 7 {
                continue;
            }

            let ref_name = parts[0].trim();
            let short_name = parts[1].trim();
            let is_head_marker = parts[2].trim() == "*";
            let upstream_raw = parts[3].trim();
            // «ahead 2, behind 5», «behind 3», «gone» o vacío
            let track = parts[4].trim();
            let count = |word: &str| {
                track
                    .split(',')
                    .map(str::trim)
                    .find_map(|t| t.strip_prefix(word).map(|n| n.trim().parse::<u32>().unwrap_or(0)))
                    .unwrap_or(0)
            };
            let (ahead, behind) = (count("ahead"), count("behind"));
            let committerdate_rel = parts[5].trim();
            let subject = parts[6].trim();

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

            let branch_item = Branch {
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
                ahead,
                behind,
            };

            if is_remote {
                remote_branches.push(branch_item);
            } else {
                local_branches.push(branch_item);
            }
        }

        Ok(BranchList {
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

        let ref_exists = |r: &str| {
            crate::infrastructure::service::os::git_binary::git_command()
                .current_dir(&work_dir)
                .args(["show-ref", "--verify", "--quiet", r])
                .status()
                .map(|st| st.success())
                .unwrap_or(false)
        };

        // «origin/foo» o «refs/remotes/origin/foo»
        let short = branch_name.strip_prefix("refs/remotes/").unwrap_or(branch_name);
        // Una rama local manda: «feat/x» es la rama local, no «x» en el remoto «feat»
        let is_local = !branch_name.starts_with("refs/remotes/") && ref_exists(&format!("refs/heads/{short}"));
        // Rama remota: separar el remoto (el más largo que coincida, por si su nombre lleva «/»)
        let remote_split = if is_local || !ref_exists(&format!("refs/remotes/{short}")) {
            None
        } else {
            let remotes = crate::infrastructure::service::os::git_binary::git_command()
                .current_dir(&work_dir)
                .arg("remote")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default();
            remotes
                .lines()
                .map(str::trim)
                .filter(|r| !r.is_empty())
                .filter_map(|r| short.strip_prefix(r).and_then(|rest| rest.strip_prefix('/')).map(|local| (r, local)))
                .filter(|(_, local)| !local.is_empty())
                .max_by_key(|(r, _)| r.len())
                .map(|(_, local)| local.to_string())
        };

        let mut args: Vec<String> = vec!["checkout".into()];
        let checkout_target: String = match remote_split {
            // Ya hay rama local con ese nombre: cambiar a ella
            Some(local) if ref_exists(&format!("refs/heads/{local}")) => {
                args.push(local.clone());
                local
            }
            // Sin rama local: crearla siguiendo a la remota (no dejar HEAD suelto)
            Some(local) => {
                args.extend(["-b".into(), local.clone(), "--track".into(), short.to_string()]);
                local
            }
            None => {
                args.push(short.to_string());
                short.to_string()
            }
        };
        let checkout_target = checkout_target.as_str();

        let output = crate::infrastructure::service::os::git_binary::git_command()
            .current_dir(&work_dir)
            .env("LC_ALL", "C")
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to execute git checkout: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("overwritten by checkout") || stderr.contains("would be overwritten")
            {
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
        let check_format = crate::infrastructure::service::os::git_binary::git_command()
            .current_dir(&work_dir)
            .args(["check-ref-format", "--branch", new_branch])
            .output()
            .map_err(|e| format!("Error al verificar formato de rama: {e}"))?;

        if !check_format.status.success() {
            return Err(format!(
                "El nombre de rama '{new_branch}' no es válido según las reglas de Git."
            ));
        }

        // Verificar si la rama ya existe localmente
        let check_exists = crate::infrastructure::service::os::git_binary::git_command()
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
                crate::infrastructure::service::os::git_binary::git_command()
                    .current_dir(&work_dir)
                    .args(["checkout", "-b", new_branch])
                    .output()
            } else {
                crate::infrastructure::service::os::git_binary::git_command()
                    .current_dir(&work_dir)
                    .args(["checkout", "-b", new_branch, base_branch])
                    .output()
            }
        } else {
            if base_branch.is_empty() {
                crate::infrastructure::service::os::git_binary::git_command()
                    .current_dir(&work_dir)
                    .args(["branch", new_branch])
                    .output()
            } else {
                crate::infrastructure::service::os::git_binary::git_command()
                    .current_dir(&work_dir)
                    .args(["branch", new_branch, base_branch])
                    .output()
            }
        }
        .map_err(|e| format!("Failed to create branch: {e}"))?;

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

    pub fn git_push(&self, vault_path: &Path) -> Result<String, String> {
        let repo =
            gix::discover(vault_path).map_err(|e| format!("No es un repositorio Git: {e}"))?;
        let work_dir = repo
            .workdir()
            .ok_or_else(|| "Bare repository not supported".to_string())?
            .to_path_buf();

        let output = crate::infrastructure::service::os::git_binary::git_command()
            .current_dir(&work_dir)
            .arg("push")
            .output()
            .map_err(|e| format!("Error al ejecutar git push: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if !stderr.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            return Err(format!("git push falló: {detail}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let out = if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            "Push completado exitosamente".to_string()
        };

        Ok(out)
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

        let git_service = VersionControlImpl::new();
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

        let git_service = VersionControlImpl::new();
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
        assert_eq!(
            status2
                .statuses
                .get("file_a.txt")
                .unwrap()
                .worktree
                .as_deref(),
            Some("M")
        );
        assert!(!status2.statuses.contains_key("file_b.txt"));

        // Test empty message error
        let err = git_service.git_commit(&temp_dir, &["file_a.txt".to_string()], "   ");
        assert!(err.is_err());

        // Test delete file_b from disk
        std::fs::remove_file(&file_b).unwrap();
        let status3 = git_service.get_vault_status(&temp_dir).unwrap();
        let stat_b_deleted = status3.statuses.get("file_b.txt").cloned().unwrap();
        assert_eq!(stat_b_deleted.worktree.as_deref(), Some("D"));

        let res_del =
            git_service.git_commit(&temp_dir, &["file_b.txt".to_string()], "commit deleted b");
        assert!(res_del.is_ok());
        let status4 = git_service.get_vault_status(&temp_dir).unwrap();
        assert!(!status4.statuses.contains_key("file_b.txt"));

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
        let git_service = VersionControlImpl::new();
        let diff_worktree = git_service
            .git_diff(&temp_dir, "file_a.txt", false, None)
            .unwrap();
        assert!(!diff_worktree.is_staged);
        assert_eq!(diff_worktree.old_content, "line 1\nline 2\n");
        assert_eq!(diff_worktree.new_content, "line 1\nline 2 mod\nline 3\n");
        assert!(diff_worktree.diff.contains("+line 2 mod"));

        // 2. Diff de stage debe estar vacío antes de add
        let diff_staged_empty = git_service
            .git_diff(&temp_dir, "file_a.txt", true, None)
            .unwrap();
        assert!(diff_staged_empty.diff.is_empty());

        // 3. Stage del cambio y comprobar diff staged
        run_cmd(&["add", "file_a.txt"]);
        let diff_staged = git_service
            .git_diff(&temp_dir, "file_a.txt", true, None)
            .unwrap();
        assert!(diff_staged.is_staged);
        assert_eq!(diff_staged.old_content, "line 1\nline 2\n");
        assert_eq!(diff_staged.new_content, "line 1\nline 2 mod\nline 3\n");
        assert!(diff_staged.diff.contains("+line 2 mod"));

        // 4. Todo está en stage: el diff de trabajo (local contra stage) queda vacío y lo marca,
        //    en vez de caer en silencio al diff contra HEAD
        let diff_work = git_service.git_diff(&temp_dir, "file_a.txt", false, None).unwrap();
        assert!(diff_work.diff.is_empty(), "{}", diff_work.diff);
        assert!(diff_work.only_staged);

        // 5. Otro cambio local encima del stage: el diff de trabajo es solo ese cambio
        std::fs::write(temp_dir.join("file_a.txt"), "line 1\nline 2 mod\nline 3\nline 4\n").unwrap();
        let diff_work = git_service.git_diff(&temp_dir, "file_a.txt", false, None).unwrap();
        assert_eq!(diff_work.old_content, "line 1\nline 2 mod\nline 3\n", "el antes es el stage");
        assert!(diff_work.diff.contains("+line 4"));
        assert!(!diff_work.diff.contains("+line 2 mod"), "lo que ya está en stage no se repite");
        assert!(!diff_work.only_staged);

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

        let git_service = VersionControlImpl::new();

        // 1. List branches - initially master or main
        let branches = git_service.get_branches(&temp_dir).unwrap();
        assert!(!branches.local_branches.is_empty());
        let current = branches.current_branch.unwrap();
        assert!(
            branches
                .local_branches
                .iter()
                .any(|b| b.name == current && b.is_current)
        );

        // 2. Create a new branch without checkout
        let res = git_service.create_branch(&temp_dir, "feature-1", &current, false);
        assert!(res.is_ok());

        let branches = git_service.get_branches(&temp_dir).unwrap();
        assert!(
            branches
                .local_branches
                .iter()
                .any(|b| b.name == "feature-1" && !b.is_current)
        );

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

    #[test]
    fn avance_rapido_de_la_rama_actual_y_de_otra_y_rechazo_si_divergen() {
        let dir = tempfile::tempdir().unwrap();
        let origen = dir.path().join("origen");
        let clon = dir.path().join("clon");
        std::fs::create_dir_all(&origen).unwrap();
        let git = |cwd: &Path, args: &[&str]| -> String {
            let out = std::process::Command::new("git").current_dir(cwd).args(args)
                .env("GIT_AUTHOR_NAME", "T").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "T").env("GIT_COMMITTER_EMAIL", "t@t")
                .output().unwrap();
            assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        git(&origen, &["init", "-q", "-b", "main"]);
        std::fs::write(origen.join("a.md"), "1").unwrap();
        git(&origen, &["add", "."]);
        git(&origen, &["commit", "-qm", "uno"]);
        git(&origen, &["branch", "otra"]);
        git(dir.path(), &["clone", "-q", origen.to_str().unwrap(), clon.to_str().unwrap()]);
        git(&clon, &["branch", "otra", "origin/otra"]);
        for n in ["2", "3"] {
            std::fs::write(origen.join("a.md"), n).unwrap();
            git(&origen, &["commit", "-qam", n]);
        }
        git(&origen, &["branch", "-f", "otra", "main"]);
        git(&clon, &["fetch", "-q"]);
        let svc = VersionControlImpl::new();

        // Contadores: main va 2 por detrás de origin/main
        let b = svc.get_branches(&clon).unwrap();
        let main = b.local_branches.iter().find(|x| x.name == "main").unwrap();
        assert_eq!((main.ahead, main.behind), (0, 2));

        // Comprobar sin tocar nada
        assert!(svc.fast_forward_branch(&clon, "main", "origin/main", true).is_ok());
        assert_eq!(git(&clon, &["rev-list", "--count", "main..origin/main"]), "2");

        // Rama actual: avanza también los archivos
        svc.fast_forward_branch(&clon, "main", "origin/main", false).unwrap();
        assert_eq!(git(&clon, &["rev-parse", "main"]), git(&clon, &["rev-parse", "origin/main"]));
        assert_eq!(std::fs::read_to_string(clon.join("a.md")).unwrap(), "3");
        assert!(svc.fast_forward_branch(&clon, "main", "origin/main", true).unwrap_err().contains("ya está"));

        // Otra rama (no la actual): solo se mueve la ref
        svc.fast_forward_branch(&clon, "otra", "origin/otra", false).unwrap();
        assert_eq!(git(&clon, &["rev-parse", "otra"]), git(&clon, &["rev-parse", "origin/otra"]));
        assert_eq!(git(&clon, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");

        // Divergen: no toca nada
        std::fs::write(clon.join("b.md"), "local").unwrap();
        git(&clon, &["add", "."]);
        git(&clon, &["commit", "-qm", "local"]);
        std::fs::write(origen.join("a.md"), "4").unwrap();
        git(&origen, &["commit", "-qam", "4"]);
        git(&clon, &["fetch", "-q"]);
        let antes = git(&clon, &["rev-parse", "main"]);
        assert!(svc.fast_forward_branch(&clon, "main", "origin/main", false).unwrap_err().contains("No es un avance rápido"));
        assert_eq!(git(&clon, &["rev-parse", "main"]), antes);
    }

    #[test]
    fn checkout_de_rama_remota_crea_la_local_con_seguimiento_y_respeta_ramas_con_barra() {
        let dir = tempfile::tempdir().unwrap();
        let origen = dir.path().join("origen");
        let clon = dir.path().join("clon");
        std::fs::create_dir_all(&origen).unwrap();
        let git = |cwd: &Path, args: &[&str]| -> String {
            let out = std::process::Command::new("git").current_dir(cwd).args(args)
                .env("GIT_AUTHOR_NAME", "T").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "T").env("GIT_COMMITTER_EMAIL", "t@t")
                .output().unwrap();
            assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        git(&origen, &["init", "-q", "-b", "main"]);
        std::fs::write(origen.join("a.md"), "1").unwrap();
        git(&origen, &["add", "."]);
        git(&origen, &["commit", "-qm", "uno"]);
        git(&origen, &["branch", "feature/x"]);
        git(dir.path(), &["clone", "-q", origen.to_str().unwrap(), clon.to_str().unwrap()]);
        let svc = VersionControlImpl::new();

        // 1. Remota sin rama local: crea feature/x siguiendo a origin/feature/x, sin HEAD suelto
        svc.checkout_branch(&clon, "origin/feature/x").unwrap();
        assert_eq!(git(&clon, &["rev-parse", "--abbrev-ref", "HEAD"]), "feature/x");
        assert_eq!(git(&clon, &["rev-parse", "--abbrev-ref", "feature/x@{upstream}"]), "origin/feature/x");

        // 2. Otra vez desde la remota: ya hay local, solo cambia a ella
        svc.checkout_branch(&clon, "main").unwrap();
        svc.checkout_branch(&clon, "refs/remotes/origin/feature/x").unwrap();
        assert_eq!(git(&clon, &["rev-parse", "--abbrev-ref", "HEAD"]), "feature/x");

        // 3. Rama local con barra: «feat/y» no salta a una local «y»
        git(&clon, &["branch", "y"]);
        git(&clon, &["branch", "feat/y"]);
        svc.checkout_branch(&clon, "feat/y").unwrap();
        assert_eq!(git(&clon, &["rev-parse", "--abbrev-ref", "HEAD"]), "feat/y");
    }

    #[test]
    fn test_git_restore_and_subdirectories() {
        let temp_dir = std::env::temp_dir().join(format!(
            "synapse_git_restore_test_{}",
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

        // Create a subfolder representing a vault inside a git repository
        let vault_dir = temp_dir.join("subvault");
        std::fs::create_dir_all(&vault_dir).unwrap();

        let file_path = vault_dir.join("doc.md");
        std::fs::write(&file_path, "initial text\n").unwrap();

        run_cmd(&["add", "subvault/doc.md"]);
        run_cmd(&["commit", "-m", "initial commit"]);

        let git_service = VersionControlImpl::new();

        // 1. Test restore worktree modification in subvault
        std::fs::write(&file_path, "changed text\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "changed text\n"
        );

        let restore_res = git_service.git_restore(&vault_dir, &["doc.md".to_string()]);
        assert!(
            restore_res.is_ok(),
            "Restore failed: {:?}",
            restore_res.err()
        );
        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "initial text\n"
        );

        // 2. Test restore staged in subvault
        std::fs::write(&file_path, "staged text\n").unwrap();
        let add_res = git_service.git_add(&vault_dir, &["doc.md".to_string()]);
        assert!(add_res.is_ok());

        let status = git_service.get_vault_status(&vault_dir).unwrap();
        let stat = status.statuses.get("doc.md").unwrap();
        assert_eq!(stat.index.as_deref(), Some("M"));

        let unstage_res = git_service.git_restore_staged(&vault_dir, &["doc.md".to_string()]);
        assert!(
            unstage_res.is_ok(),
            "Restore staged failed: {:?}",
            unstage_res.err()
        );

        let status_after = git_service.get_vault_status(&vault_dir).unwrap();
        let stat_after = status_after.statuses.get("doc.md").unwrap();
        assert_eq!(stat_after.index, None);
        assert_eq!(stat_after.worktree.as_deref(), Some("M"));

        // 3. Borrar una carpeta entera y hacer add dos veces (la segunda, con el borrado ya en stage)
        let nested = vault_dir.join("Sistema de Notas");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("Bibliografía.md"), "x\n").unwrap();
        run_cmd(&["add", "."]);
        run_cmd(&["commit", "-m", "nested"]);
        std::fs::remove_dir_all(&nested).unwrap();
        let paths = vec!["Sistema de Notas/Bibliografía.md".to_string()];
        let first = git_service.git_add(&vault_dir, &paths);
        assert!(first.is_ok(), "add de borrado falló: {:?}", first.err());
        let second = git_service.git_add(&vault_dir, &paths);
        assert!(
            second.is_ok(),
            "add de borrado ya en stage falló: {:?}",
            second.err()
        );
        let status = git_service.get_vault_status(&vault_dir).unwrap();
        let stat = status
            .statuses
            .get("Sistema de Notas/Bibliografía.md")
            .unwrap();
        assert_eq!(stat.index.as_deref(), Some("D"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_file_revision() {
        let temp_dir = std::env::temp_dir().join(format!(
            "synapse_git_revision_test_{}",
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

        let img_path = temp_dir.join("photo.png");
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG signature
        std::fs::write(&img_path, &png_bytes).unwrap();

        let diagram_path = temp_dir.join("flow.mermaid");
        std::fs::write(&diagram_path, "graph TD\n  A --> B").unwrap();

        run_cmd(&["add", "photo.png", "flow.mermaid"]);
        run_cmd(&["commit", "-m", "init commit"]);

        let git_service = VersionControlImpl::new();

        // 1. Extraer imagen desde HEAD
        let img_rev = git_service
            .get_file_revision(&temp_dir, "photo.png", "HEAD")
            .unwrap();
        assert!(img_rev.exists);
        assert!(img_rev.is_binary);
        assert!(img_rev.data_url.is_some());
        assert!(
            img_rev
                .data_url
                .unwrap()
                .starts_with("data:image/png;base64,")
        );

        // 2. Extraer texto desde HEAD
        let mmd_rev = git_service
            .get_file_revision(&temp_dir, "flow.mermaid", "HEAD")
            .unwrap();
        assert!(mmd_rev.exists);
        assert!(!mmd_rev.is_binary);
        assert_eq!(mmd_rev.content.as_deref(), Some("graph TD\n  A --> B"));

        // 3. Modificar y comprobar revisión staged vs HEAD
        std::fs::write(&diagram_path, "graph TD\n  A --> C").unwrap();
        run_cmd(&["add", "flow.mermaid"]);
        let mmd_staged = git_service
            .get_file_revision(&temp_dir, "flow.mermaid", "staged")
            .unwrap();
        assert_eq!(mmd_staged.content.as_deref(), Some("graph TD\n  A --> C"));

        // 4. Test diff perceptual de imagen en memoria
        let mut img_a = image::RgbaImage::new(10, 10);
        let mut img_b = image::RgbaImage::new(10, 10);
        for x in 0..10 {
            for y in 0..10 {
                img_a.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
                img_b.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        // Modificar 2 píxeles en img_b
        img_b.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
        img_b.put_pixel(1, 1, image::Rgba([0, 0, 0, 255]));

        let mut buf_a = std::io::Cursor::new(Vec::new());
        img_a.write_to(&mut buf_a, image::ImageFormat::Png).unwrap();
        std::fs::write(&img_path, buf_a.into_inner()).unwrap();
        run_cmd(&["add", "photo.png"]);
        run_cmd(&["commit", "-m", "update photo"]);

        let mut buf_b = std::io::Cursor::new(Vec::new());
        img_b.write_to(&mut buf_b, image::ImageFormat::Png).unwrap();
        let bytes_b = buf_b.into_inner();
        std::fs::write(&img_path, &bytes_b).unwrap();

        let image_diff_res = git_service
            .get_image_diff(&temp_dir, "photo.png", false, None)
            .unwrap();
        assert!(image_diff_res.has_differences);
        assert_eq!(image_diff_res.diff_count, 2);
        assert!(image_diff_res.diff_data_url.is_some());
        assert!(
            image_diff_res
                .diff_data_url
                .unwrap()
                .starts_with("data:image/png;base64,")
        );

        // 5. Test resolución de apuntador Git LFS simulado
        let lfs_oid = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let lfs_pointer = format!(
            "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize {}\n",
            lfs_oid,
            bytes_b.len()
        );
        // Guardar el binario real en .git/lfs/objects/01/23/...
        let lfs_obj_dir = temp_dir
            .join(".git")
            .join("lfs")
            .join("objects")
            .join("01")
            .join("23");
        std::fs::create_dir_all(&lfs_obj_dir).unwrap();
        std::fs::write(lfs_obj_dir.join(lfs_oid), &bytes_b).unwrap();

        // Commitear el apuntador LFS como si fuera un commit de Git LFS
        let lfs_img_path = temp_dir.join("lfs_photo.png");
        std::fs::write(&lfs_img_path, lfs_pointer.as_bytes()).unwrap();
        run_cmd(&["add", "lfs_photo.png"]);
        run_cmd(&["commit", "-m", "add lfs photo pointer"]);

        // Modificar localmente para generar diff
        std::fs::write(&lfs_img_path, &bytes_b).unwrap();

        let lfs_rev = git_service
            .get_file_revision(&temp_dir, "lfs_photo.png", "HEAD")
            .unwrap();
        assert!(lfs_rev.exists);
        assert!(lfs_rev.is_binary);
        let lfs_url = lfs_rev.data_url.unwrap();
        assert!(lfs_url.starts_with("data:image/png;base64,"));
        // El contenido resuelto no debe ser el texto del pointer
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
        let decoded_payload = BASE64_STANDARD
            .decode(&lfs_url["data:image/png;base64,".len()..])
            .unwrap();
        assert_eq!(decoded_payload, bytes_b);

        // 6. Test get_file_log y diff con revisión específica
        let log_res = git_service
            .get_file_log(&temp_dir, "photo.png", None)
            .unwrap();
        assert_eq!(log_res.commits.len(), 2);
        assert_eq!(log_res.commits[0].message, "update photo");
        assert_eq!(log_res.commits[1].message, "init commit");

        // Diff contra el commit "update photo" usando su hash
        let update_commit_hash = &log_res.commits[0].commit_hash;
        let diff_with_rev = git_service
            .get_image_diff(&temp_dir, "photo.png", false, Some(update_commit_hash))
            .unwrap();
        assert!(diff_with_rev.has_differences);
        assert_eq!(diff_with_rev.diff_count, 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_git_clone_repository() {
        let origin_dir = std::env::temp_dir().join(format!(
            "synapse_git_origin_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let clone_dir = std::env::temp_dir().join(format!(
            "synapse_git_clone_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&origin_dir).unwrap();

        let run_cmd = |dir: &Path, args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .status()
                .unwrap();
            assert!(status.success());
        };

        run_cmd(&origin_dir, &["init"]);
        run_cmd(&origin_dir, &["config", "user.name", "Test User"]);
        run_cmd(&origin_dir, &["config", "user.email", "test@example.com"]);
        std::fs::write(origin_dir.join("test.txt"), "hello world").unwrap();
        run_cmd(&origin_dir, &["add", "test.txt"]);
        run_cmd(&origin_dir, &["commit", "-m", "init clone repo"]);

        let git_service = VersionControlImpl::new();
        let clone_url = origin_dir.to_string_lossy().to_string();
        let res = git_service.clone_repository(&clone_url, &clone_dir);
        assert!(res.is_ok());
        assert!(clone_dir.join("test.txt").exists());

        // Test invalid url
        let empty_res = git_service.clone_repository("", &clone_dir);
        assert!(empty_res.is_err());

        let _ = std::fs::remove_dir_all(&origin_dir);
        let _ = std::fs::remove_dir_all(&clone_dir);
    }
}

/// Extrae los nombres de tag de la salida `%D` de `git log` («tag: v1.0, tag: v2» → `["v1.0", "v2"]`).
fn parse_tag_refs(refs: &str) -> Vec<String> {
    refs.split(", ")
        .filter_map(|r| r.trim().strip_prefix("tag: "))
        .map(|t| t.to_string())
        .collect()
}

#[cfg(test)]
mod tag_refs_tests {

    /// Repositorio temporal donde `nota.md` se llamaba `viejo/nota.md` antes de un renombre.
    fn renamed_repo() -> (tempfile::TempDir, String, String) {
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .current_dir(dir.path())
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok, "git {args:?}");
        };
        let rev = |args: &[&str]| {
            String::from_utf8(
                std::process::Command::new("git")
                    .current_dir(dir.path())
                    .args(args)
                    .output()
                    .unwrap()
                    .stdout,
            )
            .unwrap()
            .trim()
            .to_string()
        };
        git(&["init", "-q"]);
        std::fs::create_dir_all(dir.path().join("viejo")).unwrap();
        std::fs::write(dir.path().join("viejo/nota.md"), "uno\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "primera"]);
        let first = rev(&["rev-parse", "HEAD"]);
        std::fs::create_dir_all(dir.path().join("nuevo")).unwrap();
        git(&["mv", "viejo/nota.md", "nuevo/nota.md"]);
        git(&["commit", "-qm", "mover"]);
        std::fs::write(dir.path().join("nuevo/nota.md"), "uno\ndos\n").unwrap();
        git(&["commit", "-qam", "editar"]);
        let renamed = rev(&["rev-parse", "HEAD~1"]);
        (dir, first, renamed)
    }

    #[test]
    fn lee_la_revision_con_el_nombre_que_tenia_antes_del_renombre() {
        let (dir, first, renamed) = renamed_repo();
        let vc = super::VersionControlImpl;
        // En el primer commit el archivo se llamaba viejo/nota.md
        let rev = vc
            .get_file_revision(dir.path(), "nuevo/nota.md", &first)
            .unwrap();
        assert!(rev.exists);
        assert_eq!(rev.content.as_deref(), Some("uno\n"));
        // En el commit del renombre ya tenía el nombre nuevo
        assert!(
            vc.get_file_revision(dir.path(), "nuevo/nota.md", &renamed)
                .unwrap()
                .exists
        );
        // Diff con local contra el commit viejo: compara con su contenido, no como archivo nuevo
        let diff = vc
            .git_diff(dir.path(), "nuevo/nota.md", false, Some(&first))
            .unwrap();
        assert_eq!(diff.old_content, "uno\n");
        assert!(diff.diff.contains("+dos"), "{}", diff.diff);
        assert!(!diff.diff.contains("+uno"), "{}", diff.diff);
    }

    use super::parse_tag_refs;

    #[test]
    fn extrae_los_tags() {
        assert_eq!(parse_tag_refs("tag: v1.0, tag: v2"), vec!["v1.0", "v2"]);
        assert!(parse_tag_refs("").is_empty());
    }
}

/// Log de todas las ramas del repositorio y detalles de un commit, con la CLI de git.
mod repo_log {
    use super::{CommitDetails, CommitFileChange, CommitRef, RepoCommit, RepoLog};
    use std::path::Path;

    fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
        let out = crate::infrastructure::service::os::git_binary::git_command()
            .current_dir(dir)
            .args(args)
            .output()
            .map_err(|e| format!("Error ejecutando git: {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// Referencias de `%D` con `--decorate=full`: ramas locales, remotas y tags.
    pub(super) fn parse_refs(decoration: &str) -> Vec<CommitRef> {
        let mut refs = Vec::new();
        for raw in decoration.split(", ").map(str::trim).filter(|r| !r.is_empty()) {
            let (is_head, r) = match raw.strip_prefix("HEAD -> ") {
                Some(rest) => (true, rest),
                None => (false, raw),
            };
            if r == "HEAD" {
                continue;
            }
            let r = r.strip_prefix("tag: ").unwrap_or(r);
            let (kind, name) = if let Some(n) = r.strip_prefix("refs/heads/") {
                ("local", n)
            } else if let Some(n) = r.strip_prefix("refs/remotes/") {
                if n.ends_with("/HEAD") {
                    continue;
                }
                ("remote", n)
            } else if let Some(n) = r.strip_prefix("refs/tags/") {
                ("tag", n)
            } else {
                continue;
            };
            refs.push(CommitRef { name: name.to_string(), kind: kind.to_string(), is_head });
        }
        // La rama de HEAD primero, luego locales, remotas y tags
        refs.sort_by_key(|r| (!r.is_head, match r.kind.as_str() { "local" => 0, "remote" => 1, _ => 2 }));
        refs
    }

    pub fn get_repo_log(dir: &Path, max_count: Option<usize>) -> Result<RepoLog, String> {
        let limit = max_count.unwrap_or(1000).to_string();
        let head = git(dir, &["rev-parse", "--verify", "-q", "HEAD"])
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        // Un repositorio sin commits no tiene log
        if head.is_none() && git(dir, &["rev-list", "-n", "1", "--all"]).map(|s| s.trim().is_empty()).unwrap_or(true) {
            return Ok(RepoLog { head: None, commits: Vec::new() });
        }
        let out = git(
            dir,
            &[
                "log",
                "--exclude=refs/stash",
                "--all",
                "--date-order",
                "--decorate=full",
                "--format=\x1e%H\x1f%h\x1f%P\x1f%an\x1f%ae\x1f%aI\x1f%at\x1f%D\x1f%s",
                "-n",
                &limit,
            ],
        )?;
        let commits = out
            .split('\x1e')
            .filter_map(|block| {
                let p: Vec<&str> = block.trim_end_matches('\n').splitn(9, '\x1f').collect();
                if p.len() < 9 {
                    return None;
                }
                Some(RepoCommit {
                    commit_hash: p[0].trim().to_string(),
                    short_hash: p[1].to_string(),
                    parents: p[2].split_whitespace().map(str::to_string).collect(),
                    author_name: p[3].to_string(),
                    author_email: p[4].to_string(),
                    date: p[5].to_string(),
                    timestamp: p[6].parse().unwrap_or(0),
                    refs: parse_refs(p[7]),
                    message: p[8].trim().to_string(),
                })
            })
            .collect();
        Ok(RepoLog { head, commits })
    }

    pub fn get_commit_details(dir: &Path, commit: &str) -> Result<CommitDetails, String> {
        if commit.is_empty() || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Commit no válido: {commit}"));
        }
        let header = git(
            dir,
            &["show", "-s", "--format=%H\x1f%P\x1f%an\x1f%ae\x1f%cn\x1f%aI\x1f%B", commit],
        )?;
        let p: Vec<&str> = header.splitn(7, '\x1f').collect();
        if p.len() < 7 {
            return Err(format!("No se pudo leer el commit {commit}"));
        }
        let hash = p[0].trim().to_string();
        let parents: Vec<String> = p[1].split_whitespace().map(str::to_string).collect();
        // Cambios respecto al primer padre (o todo el árbol en el commit raíz)
        let names = match parents.first() {
            Some(parent) => git(dir, &["diff-tree", "-r", "-M", "--name-status", "-z", parent, &hash])?,
            None => git(dir, &["diff-tree", "--root", "--no-commit-id", "-r", "-M", "--name-status", "-z", &hash])?,
        };
        Ok(CommitDetails {
            commit_hash: hash,
            parents,
            author_name: p[2].to_string(),
            author_email: p[3].to_string(),
            committer_name: p[4].to_string(),
            date: p[5].to_string(),
            message: p[6].trim().to_string(),
            files: parse_name_status_z(&names),
        })
    }

    /// Nombre de ref válido para git (y que no se confunda con una opción).
    fn check_ref_name(dir: &Path, kind: &str, name: &str) -> Result<(), String> {
        let full = format!("refs/{kind}/{name}");
        if name.is_empty() || name.starts_with('-') || git(dir, &["check-ref-format", &full]).is_err() {
            return Err(format!("«{name}» no es un nombre válido"));
        }
        Ok(())
    }

    pub fn create_tag(dir: &Path, name: &str, commit: &str, message: Option<&str>) -> Result<String, String> {
        check_ref_name(dir, "tags", name)?;
        if commit.is_empty() || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Commit no válido: {commit}"));
        }
        match message {
            Some(m) => git(dir, &["tag", "-a", name, "-m", m, commit]),
            None => git(dir, &["tag", name, commit]),
        }
        .map_err(|e| format!("No se pudo crear el tag: {e}"))?;
        Ok(format!("Tag «{name}» creado"))
    }

    pub fn delete_tag(dir: &Path, name: &str) -> Result<String, String> {
        check_ref_name(dir, "tags", name)?;
        git(dir, &["tag", "-d", name]).map_err(|e| format!("No se pudo borrar el tag: {e}"))?;
        Ok(format!("Tag «{name}» borrado"))
    }

    /// Avance rápido de una rama local hasta `target` (commit o rama, p. ej. `origin/main`). Si la
    /// rama es la actual se usa `merge --ff-only` (actualiza los archivos); si no, solo se mueve la ref.
    /// Nunca crea commits ni descarta trabajo: si las ramas divergieron, no toca nada.
    pub fn fast_forward_branch(dir: &Path, branch: &str, target: &str, dry_run: bool) -> Result<String, String> {
        check_ref_name(dir, "heads", branch)?;
        let target = target.trim();
        if target.is_empty() || target.starts_with('-') {
            return Err(format!("Destino no válido: {target}"));
        }
        let rev = |r: &str| git(dir, &["rev-parse", "--verify", "--quiet", &format!("{r}^{{commit}}")]).map(|s| s.trim().to_string());
        let old = rev(&format!("refs/heads/{branch}")).map_err(|_| format!("No existe la rama local «{branch}»"))?;
        let new = rev(target).map_err(|_| format!("No se encontró «{target}»"))?;
        if old == new {
            return Err(format!("«{branch}» ya está en ese commit"));
        }
        if git(dir, &["merge-base", "--is-ancestor", &new, &old]).is_ok() {
            return Err(format!("«{branch}» ya incluye ese commit (está por delante)"));
        }
        if git(dir, &["merge-base", "--is-ancestor", &old, &new]).is_err() {
            return Err(format!(
                "No es un avance rápido: «{branch}» tiene commits que no están en «{target}». Hay que hacer rebase o merge."
            ));
        }
        if dry_run {
            return Ok(String::new());
        }
        let current = git(dir, &["symbolic-ref", "--quiet", "--short", "HEAD"]).map(|s| s.trim().to_string()).ok();
        if current.as_deref() == Some(branch) {
            git(dir, &["merge", "--ff-only", "--quiet", &new]).map_err(|e| format!("No se pudo avanzar «{branch}»: {e}"))?;
        } else {
            git(dir, &["update-ref", "-m", &format!("synapse: fast-forward a {target}"), &format!("refs/heads/{branch}"), &new, &old])
                .map_err(|e| format!("No se pudo avanzar «{branch}»: {e}"))?;
        }
        let n = git(dir, &["rev-list", "--count", &format!("{old}..{new}")]).map(|s| s.trim().to_string()).unwrap_or_default();
        Ok(format!("«{branch}» avanzó {n} commit(s) hasta {}", &new[..new.len().min(7)]))
    }

    pub fn delete_branch(dir: &Path, name: &str, force: bool) -> Result<String, String> {
        check_ref_name(dir, "heads", name)?;
        git(dir, &["branch", if force { "-D" } else { "-d" }, name]).map_err(|e| {
            if e.contains("not fully merged") {
                format!("La rama «{name}» tiene commits sin fusionar")
            } else {
                format!("No se pudo borrar la rama: {e}")
            }
        })?;
        Ok(format!("Rama «{name}» borrada"))
    }

    /// Salida de `--name-status -z`: `M\0ruta\0`, `R090\0vieja\0nueva\0`.
    pub(super) fn parse_name_status_z(out: &str) -> Vec<CommitFileChange> {
        let mut it = out.split('\0').filter(|s| !s.is_empty());
        let mut files = Vec::new();
        while let Some(status) = it.next() {
            let kind = &status[..1];
            if kind == "R" || kind == "C" {
                let (Some(old), Some(new)) = (it.next(), it.next()) else { break };
                files.push(CommitFileChange {
                    path: new.to_string(),
                    change: kind.to_string(),
                    previous_path: Some(old.to_string()),
                });
            } else if let Some(path) = it.next() {
                files.push(CommitFileChange { path: path.to_string(), change: kind.to_string(), previous_path: None });
            }
        }
        files
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn clasifica_refs_y_pone_head_primero() {
            let refs = parse_refs("HEAD -> refs/heads/main, tag: refs/tags/v1, refs/remotes/origin/main, refs/remotes/origin/HEAD, refs/heads/dev");
            let names: Vec<_> = refs.iter().map(|r| (r.name.as_str(), r.kind.as_str(), r.is_head)).collect();
            assert_eq!(
                names,
                vec![("main", "local", true), ("dev", "local", false), ("origin/main", "remote", false), ("v1", "tag", false)]
            );
        }

        #[test]
        fn lee_name_status_con_renombres() {
            let f = parse_name_status_z("M\0a.md\0R100\0viejo.md\0nuevo.md\0A\0b.md\0");
            assert_eq!(f.len(), 3);
            assert_eq!(f[1].path, "nuevo.md");
            assert_eq!(f[1].previous_path.as_deref(), Some("viejo.md"));
            assert_eq!(f[2].change, "A");
        }

        #[test]
        fn log_de_todas_las_ramas_con_fusion() {
            let dir = tempfile::tempdir().unwrap();
            let d = dir.path();
            let g = |args: &[&str]| {
                let ok = std::process::Command::new("git").current_dir(d).args(args)
                    .env("GIT_AUTHOR_NAME", "T").env("GIT_AUTHOR_EMAIL", "t@t").env("GIT_COMMITTER_NAME", "T").env("GIT_COMMITTER_EMAIL", "t@t")
                    .status().unwrap().success();
                assert!(ok, "git {args:?}");
            };
            g(&["init", "-q", "-b", "main"]);
            // create_tag llama a git sin las variables de g: un tag anotado necesita identidad en el repo
            g(&["config", "user.name", "T"]);
            g(&["config", "user.email", "t@t"]);
            std::fs::write(d.join("a.md"), "1").unwrap();
            g(&["add", "."]);
            g(&["commit", "-qm", "uno"]);
            g(&["checkout", "-qb", "dev"]);
            std::fs::write(d.join("b.md"), "2").unwrap();
            g(&["add", "."]);
            g(&["commit", "-qm", "dos"]);
            g(&["tag", "v1"]);
            g(&["checkout", "-q", "main"]);
            std::fs::write(d.join("a.md"), "3").unwrap();
            g(&["commit", "-qam", "tres"]);
            g(&["merge", "-q", "--no-ff", "-m", "fusión", "dev"]);

            let log = get_repo_log(d, None).unwrap();
            assert_eq!(log.commits.len(), 4);
            assert_eq!(log.commits[0].message, "fusión");
            assert_eq!(log.commits[0].parents.len(), 2);
            assert_eq!(log.head.as_deref(), Some(log.commits[0].commit_hash.as_str()));
            let dos = log.commits.iter().find(|c| c.message == "dos").unwrap();
            assert!(dos.refs.iter().any(|r| r.name == "dev" && r.kind == "local"));
            assert!(dos.refs.iter().any(|r| r.name == "v1" && r.kind == "tag"));

            let det = get_commit_details(d, &dos.commit_hash).unwrap();
            assert_eq!(det.message, "dos");
            assert_eq!(det.files, vec![CommitFileChange { path: "b.md".into(), change: "A".into(), previous_path: None }]);
            let raiz = log.commits.last().unwrap();
            assert_eq!(get_commit_details(d, &raiz.commit_hash).unwrap().files.len(), 1);
            assert!(get_commit_details(d, "--all").is_err());

            // Tags y ramas
            assert!(create_tag(d, "v2", &raiz.commit_hash, Some("anotado")).is_ok());
            assert!(create_tag(d, "-x", &raiz.commit_hash, None).is_err());
            assert!(create_tag(d, "v3", "HEAD", None).is_err());
            let log = get_repo_log(d, None).unwrap();
            assert!(log.commits.last().unwrap().refs.iter().any(|r| r.name == "v2" && r.kind == "tag"));
            assert!(delete_tag(d, "v2").is_ok());
            g(&["branch", "suelta", &raiz.commit_hash]);
            g(&["checkout", "-qb", "sin-fusionar"]);
            std::fs::write(d.join("c.md"), "x").unwrap();
            g(&["add", "."]);
            g(&["commit", "-qm", "suelto"]);
            g(&["checkout", "-q", "main"]);
            assert!(delete_branch(d, "suelta", false).is_ok());
            assert!(delete_branch(d, "sin-fusionar", false).unwrap_err().contains("sin fusionar"));
            assert!(delete_branch(d, "sin-fusionar", true).is_ok());
        }
    }
}
