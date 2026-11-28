//! Fusión con la CLI de git. Todo el estado se lee del repositorio, así que sobrevive a reinicios y
//! a lo que se haga desde la terminal.

use super::version_control_impl::repo_log::{commit_id, current_branch, git, has_state, nothing_in_progress};
use crate::domain::version::history_change::UndoPoint;
use crate::domain::version::merge::{
    ConflictVersions, MergeAction, MergeConflict, MergeOutcome, MergeState, Resolution,
};
use crate::infrastructure::service::os::git_binary::git_command;
use std::path::{Path, PathBuf};

/// Una entrada sin fusionar del índice.
struct Stage {
    path: String,
    mode: String,
    oid: String,
    stage: u8,
}

fn root(dir: &Path) -> Result<PathBuf, String> {
    Ok(PathBuf::from(git(dir, &["rev-parse", "--show-toplevel"])?.trim()))
}

fn unmerged(root: &Path) -> Result<Vec<Stage>, String> {
    let out = git(root, &["ls-files", "-u", "-z"])?;
    Ok(out
        .split('\0')
        .filter_map(|entry| {
            let (meta, path) = entry.split_once('\t')?;
            let mut meta = meta.split(' ');
            Some(Stage {
                path: path.to_string(),
                mode: meta.next()?.to_string(),
                oid: meta.next()?.to_string(),
                stage: meta.next()?.parse().ok()?,
            })
        })
        .collect())
}

/// El texto de una versión, tal como quedaría en disco; `None` si no es un archivo de texto.
fn text_of(root: &Path, stage: &Stage) -> Option<String> {
    // Los enlaces simbólicos y los submódulos no se fusionan por líneas
    if !stage.mode.starts_with("100") {
        return None;
    }
    let out = git_command()
        .current_dir(root)
        .args(["cat-file", "--filters", &format!("--path={}", stage.path), &stage.oid])
        .output()
        .ok()?;
    if !out.status.success() || out.stdout.contains(&0) {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

fn versions_of(root: &Path, stages: &[Stage], path: &str) -> ConflictVersions {
    let mut versions = ConflictVersions::default();
    for stage in stages.iter().filter(|s| s.path == path) {
        let text = text_of(root, stage);
        versions.binary |= text.is_none();
        match stage.stage {
            1 => versions.base = text,
            2 => versions.ours = text,
            _ => versions.theirs = text,
        }
    }
    versions
}

/// El nombre de lo que se fusiona: el que git puso en el mensaje, o el hash corto.
fn incoming_name(root: &Path, message: &str) -> String {
    let first = message.lines().next().unwrap_or_default();
    let quoted = first.split_once('\'').and_then(|(_, rest)| rest.split_once('\''));
    match quoted {
        Some((name, _)) => name.to_string(),
        None => git(root, &["rev-parse", "--short", "MERGE_HEAD"])
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
    }
}

pub fn state(dir: &Path) -> Result<Option<MergeState>, String> {
    let root = root(dir)?;
    if !has_state(&root, "MERGE_HEAD") {
        return Ok(None);
    }
    let message_file = git(&root, &["rev-parse", "--git-path", "MERGE_MSG"])?;
    let raw = std::fs::read_to_string(root.join(message_file.trim())).unwrap_or_default();
    // Las líneas `#` son la lista de conflictos que git no incluye en el commit
    let message = raw
        .lines()
        .filter(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    let stages = unmerged(&root)?;
    let mut conflicts: Vec<MergeConflict> = Vec::new();
    for stage in &stages {
        if conflicts.last().is_none_or(|c| c.path != stage.path) {
            conflicts.push(MergeConflict {
                path: stage.path.clone(),
                base: false,
                ours: false,
                theirs: false,
                binary: false,
            });
        }
        let Some(conflict) = conflicts.last_mut() else { continue };
        conflict.binary |= text_of(&root, stage).is_none();
        match stage.stage {
            1 => conflict.base = true,
            2 => conflict.ours = true,
            _ => conflict.theirs = true,
        }
    }
    Ok(Some(MergeState {
        ours: current_branch(&root).unwrap_or_else(|_| "HEAD".to_string()),
        theirs: incoming_name(&root, &message),
        message,
        conflicts,
    }))
}

pub fn versions(dir: &Path, path: &str) -> Result<ConflictVersions, String> {
    let root = root(dir)?;
    let stages = unmerged(&root)?;
    if !stages.iter().any(|s| s.path == path) {
        return Err(format!("{path}: not conflicted"));
    }
    Ok(versions_of(&root, &stages, path))
}

fn merged(branch: String, before: String, after: String) -> MergeOutcome {
    MergeOutcome::Merged { undo: UndoPoint { branch, before, after, stash: None, files: true } }
}

fn start(dir: &Path, target: &str) -> Result<MergeOutcome, String> {
    let branch = current_branch(dir)?;
    nothing_in_progress(dir)?;
    // Comprueba que existe (y que no es una opción); a git se le pasa el nombre para el mensaje
    commit_id(dir, target)?;
    let before = commit_id(dir, "HEAD")?;
    // Sin autostash: reaplicarlo podría dar conflictos que no se verían aquí
    let result = git(dir, &["merge", "--no-edit", "--no-squash", "--commit", "--no-autostash", target.trim()]);
    if let Err(error) = result {
        if has_state(dir, "MERGE_HEAD") && !unmerged(&root(dir)?)?.is_empty() {
            return Ok(MergeOutcome::Conflicts);
        }
        // P. ej. cambios locales que se pisarían, o un hook que rechaza el commit
        return Err(format!("Could not merge: {error}"));
    }
    let after = commit_id(dir, "HEAD")?;
    Ok(if after == before { MergeOutcome::UpToDate } else { merged(branch, before, after) })
}

fn resolve(dir: &Path, path: &str, resolution: &Resolution) -> Result<MergeOutcome, String> {
    let root = root(dir)?;
    if !has_state(&root, "MERGE_HEAD") {
        return Err("no merge in progress".to_string());
    }
    let stages = unmerged(&root)?;
    // Solo rutas que git da como en conflicto: la ruta llega de la interfaz
    let of_path: Vec<&Stage> = stages.iter().filter(|s| s.path == path).collect();
    if of_path.is_empty() {
        return Err(format!("{path}: not conflicted"));
    }
    let literal = |args: &[&str]| -> Result<String, String> {
        let mut all = vec!["--literal-pathspecs"];
        all.extend_from_slice(args);
        git(&root, &all)
    };
    let wanted = match resolution {
        Resolution::Content { content } => {
            let file = root.join(path);
            let is_link = std::fs::symlink_metadata(&file).is_ok_and(|m| m.file_type().is_symlink());
            if is_link || of_path.iter().any(|s| !s.mode.starts_with("100")) {
                return Err(format!("{path}: not a text file: choose one of the versions"));
            }
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&file, content).map_err(|e| format!("Could not write «{path}»: {e}"))?;
            literal(&["add", "-f", "--", path])?;
            return Ok(MergeOutcome::Resolved);
        }
        Resolution::Ours => of_path.iter().find(|s| s.stage == 2),
        Resolution::Theirs => of_path.iter().find(|s| s.stage == 3),
        Resolution::Delete => None,
    };
    match wanted {
        Some(stage) => {
            let entry = format!("{},{},{}", stage.mode, stage.oid, path);
            git(&root, &["update-index", "--add", "--cacheinfo", &entry])?;
            // De un submódulo solo cambia el índice
            if stage.mode != "160000" {
                git(&root, &["checkout-index", "-f", "--", path])?;
            }
        }
        None => {
            literal(&["rm", "-q", "-f", "--", path])?;
        }
    }
    Ok(MergeOutcome::Resolved)
}

fn conclude(dir: &Path, message: Option<&str>) -> Result<MergeOutcome, String> {
    let root = root(dir)?;
    if !has_state(&root, "MERGE_HEAD") {
        return Err("no merge in progress".to_string());
    }
    if !unmerged(&root)?.is_empty() {
        return Err("unresolved conflicts remain".to_string());
    }
    let branch = current_branch(&root)?;
    let before = commit_id(&root, "HEAD")?;
    let mut args = vec!["commit"];
    match message.map(str::trim).filter(|m| !m.is_empty()) {
        Some(m) => args.extend(["-m", m]),
        None => args.push("--no-edit"),
    }
    git(&root, &args).map_err(|e| format!("Could not commit merge: {e}"))?;
    let after = commit_id(&root, "HEAD")?;
    Ok(merged(branch, before, after))
}

pub fn act(dir: &Path, action: &MergeAction) -> Result<MergeOutcome, String> {
    match action {
        MergeAction::Start { target } => start(dir, target),
        MergeAction::Resolve { path, resolution } => resolve(dir, path, resolution),
        MergeAction::Continue { message } => conclude(dir, message.as_deref()),
        MergeAction::Abort => {
            if !has_state(dir, "MERGE_HEAD") {
                return Err("no merge in progress".to_string());
            }
            git(dir, &["merge", "--abort"]).map_err(|e| format!("Could not abort merge: {e}"))?;
            Ok(MergeOutcome::Aborted)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Repo(tempfile::TempDir);

    impl Repo {
        /// `main` y `otra` divergen: ambas cambian la línea 2 de `a.txt`; `otra` además la 6.
        fn diverged() -> Self {
            let repo = Self(tempfile::tempdir().unwrap());
            repo.sh("git init -q -b main; git config user.name t; git config user.email t@t; git config commit.gpgsign false");
            repo.sh("printf '1\\n2\\n3\\n4\\n5\\n6\\n' > a.txt; echo del > mod.txt; echo del2 > mod2.txt; printf 'a\\0b' > bin.dat; git add .; git commit -qm base");
            repo.sh("git checkout -q -b otra; printf '1\\nsuyo\\n3\\n4\\n5\\nseis\\n' > a.txt; echo cambiado > mod.txt; git rm -q mod2.txt; printf 'a\\0c' > bin.dat; echo suyo > 'nuevo[1].txt'; git add .; git commit -qm suyo");
            repo.sh("git checkout -q main; printf '1\\nmio\\n3\\n4\\n5\\n6\\n' > a.txt; git rm -q mod.txt; echo cambiado > mod2.txt; printf 'a\\0d' > bin.dat; echo mio > 'nuevo[1].txt'; git add .; git commit -qm mio");
            repo
        }
        fn path(&self) -> &Path {
            self.0.path()
        }
        fn sh(&self, script: &str) {
            let status = std::process::Command::new("sh").args(["-ec", script]).current_dir(self.path()).status().unwrap();
            assert!(status.success(), "{script}");
        }
        fn read(&self, file: &str) -> String {
            std::fs::read_to_string(self.path().join(file)).unwrap()
        }
        fn head(&self) -> String {
            commit_id(self.path(), "HEAD").unwrap()
        }
        fn start(&self, target: &str) -> Result<MergeOutcome, String> {
            act(self.path(), &MergeAction::Start { target: target.into() })
        }
        fn resolve(&self, path: &str, resolution: Resolution) -> Result<MergeOutcome, String> {
            act(self.path(), &MergeAction::Resolve { path: path.into(), resolution })
        }
        fn conflicts(&self) -> Vec<MergeConflict> {
            state(self.path()).unwrap().unwrap().conflicts
        }
    }

    #[test]
    fn fusion_sin_conflictos_crea_el_commit_y_se_puede_deshacer() {
        let repo = Repo::diverged();
        repo.sh("git checkout -q -b limpia HEAD~1; echo x > otro.txt; git add .; git commit -qm otro; git checkout -q main");
        let before = repo.head();
        let MergeOutcome::Merged { undo } = repo.start("limpia").unwrap() else { panic!("sin fusionar") };
        assert_eq!(undo.before, before);
        assert!(commit_id(repo.path(), "HEAD^2").is_ok());
        assert!(state(repo.path()).unwrap().is_none());
        assert_eq!(repo.start("limpia").unwrap(), MergeOutcome::UpToDate);
        super::super::version_control_impl::repo_log::change_history(
            repo.path(),
            &crate::domain::version::history_change::HistoryAction::Undo { point: undo },
        )
        .unwrap();
        assert_eq!(repo.head(), before);
    }

    #[test]
    fn conflictos_versiones_resolucion_y_commit() {
        let repo = Repo::diverged();
        assert_eq!(repo.start("otra").unwrap(), MergeOutcome::Conflicts);
        let st = state(repo.path()).unwrap().unwrap();
        assert_eq!((st.ours.as_str(), st.theirs.as_str()), ("main", "otra"));
        assert_eq!(st.message, "Merge branch 'otra'");
        let summary: Vec<_> = st.conflicts.iter().map(|c| (c.path.as_str(), c.base, c.ours, c.theirs, c.binary)).collect();
        assert_eq!(
            summary,
            vec![
                ("a.txt", true, true, true, false),
                ("bin.dat", true, true, true, true),
                ("mod.txt", true, false, true, false),
                ("mod2.txt", true, true, false, false),
                ("nuevo[1].txt", false, true, true, false),
            ]
        );
        let v = versions(repo.path(), "a.txt").unwrap();
        assert_eq!(v.base.as_deref(), Some("1\n2\n3\n4\n5\n6\n"));
        assert_eq!(v.ours.as_deref(), Some("1\nmio\n3\n4\n5\n6\n"));
        assert_eq!(v.theirs.as_deref(), Some("1\nsuyo\n3\n4\n5\nseis\n"));
        assert_eq!(versions(repo.path(), "nuevo[1].txt").unwrap().base, None);

        assert!(act(repo.path(), &MergeAction::Continue { message: None }).unwrap_err().contains("unresolved conflicts"));
        assert!(repo.resolve("../fuera.txt", Resolution::Content { content: "x".into() }).is_err());
        assert!(repo.resolve(".git/config", Resolution::Content { content: "x".into() }).is_err());

        repo.resolve("a.txt", Resolution::Content { content: "1\nambos\n3\n4\n5\nseis\n".into() }).unwrap();
        repo.resolve("bin.dat", Resolution::Theirs).unwrap();
        // En la rama actual `mod.txt` está borrado: «la mía» es borrarlo
        repo.resolve("mod.txt", Resolution::Ours).unwrap();
        repo.resolve("mod2.txt", Resolution::Delete).unwrap();
        repo.resolve("nuevo[1].txt", Resolution::Theirs).unwrap();
        assert!(repo.conflicts().is_empty());
        assert!(matches!(
            act(repo.path(), &MergeAction::Continue { message: Some("fusionada".into()) }).unwrap(),
            MergeOutcome::Merged { .. }
        ));
        assert_eq!(repo.read("a.txt"), "1\nambos\n3\n4\n5\nseis\n");
        assert_eq!(std::fs::read(repo.path().join("bin.dat")).unwrap(), b"a\0c");
        assert_eq!(repo.read("nuevo[1].txt"), "suyo\n");
        assert!(!repo.path().join("mod.txt").exists() && !repo.path().join("mod2.txt").exists());
        assert!(commit_id(repo.path(), "HEAD^2").is_ok());
        assert_eq!(git(repo.path(), &["log", "-1", "--format=%s"]).unwrap().trim(), "fusionada");
        assert_eq!(git(repo.path(), &["status", "--porcelain"]).unwrap(), "");
    }

    #[test]
    fn cancelar_devuelve_la_rama_y_los_archivos() {
        let repo = Repo::diverged();
        let before = repo.head();
        repo.start("otra").unwrap();
        assert!(repo.start("otra").unwrap_err().contains("in progress"));
        assert_eq!(act(repo.path(), &MergeAction::Abort).unwrap(), MergeOutcome::Aborted);
        assert_eq!(repo.head(), before);
        assert_eq!(repo.read("a.txt"), "1\nmio\n3\n4\n5\n6\n");
        assert!(state(repo.path()).unwrap().is_none());
        assert!(act(repo.path(), &MergeAction::Abort).is_err());
    }

    #[test]
    fn con_cambios_que_se_pisarian_no_empieza() {
        let repo = Repo::diverged();
        repo.sh("echo sin-confirmar > a.txt");
        assert!(repo.start("otra").is_err());
        assert!(state(repo.path()).unwrap().is_none());
        assert_eq!(repo.read("a.txt"), "sin-confirmar\n");
        assert!(repo.start("--abort").is_err());
        assert!(repo.start("no-existe").is_err());
    }
}
