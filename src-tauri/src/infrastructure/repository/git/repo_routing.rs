//! Varios repositorios Git en una bóveda: detección de los repositorios de subcarpetas y
//! enrutamiento de cada ruta (relativa a la bóveda) al repositorio más cercano que la contiene.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Carpetas que no se recorren buscando repositorios (además de las ocultas).
const SKIP_DIRS: &[&str] = &[
    "node_modules", "target", "dist", "build", "out", "coverage", "vendor", "bin", "obj",
    "bower_components", "__pycache__",
];
/// Profundidad máxima de búsqueda de repositorios dentro de la bóveda.
const MAX_DEPTH: usize = 6;
/// La lista de repositorios se reutiliza este tiempo: el estado de git se refresca a menudo.
const CACHE_TTL: Duration = Duration::from_secs(3);

/// Un repositorio encontrado: la carpeta (relativa a la bóveda) y la ruta dentro de él.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// Carpeta del repositorio relativa a la bóveda, `""` para la raíz.
    pub prefix: String,
    /// Ruta del archivo relativa a esa carpeta.
    pub inner: String,
}

impl Route {
    /// Carpeta absoluta que hace de «bóveda» para el repositorio.
    pub fn root(&self, vault: &Path) -> PathBuf {
        if self.prefix.is_empty() {
            vault.to_path_buf()
        } else {
            vault.join(&self.prefix)
        }
    }
}

fn normalize(rel: &str) -> String {
    rel.replace('\\', "/").trim_matches('/').to_string()
}

fn has_git(dir: &Path) -> bool {
    // `.git` es una carpeta en un repositorio normal y un archivo en submódulos y worktrees.
    dir.join(".git").exists()
}

/// Repositorio más cercano que contiene `rel` (la propia ruta si es una carpeta-repositorio).
/// Sin repositorio en subcarpetas, la raíz de la bóveda (git lo busca hacia arriba).
pub fn route(vault: &Path, rel: &str) -> Route {
    let rel = normalize(rel);
    let parts: Vec<&str> = if rel.is_empty() { Vec::new() } else { rel.split('/').collect() };
    for end in (1..=parts.len()).rev() {
        let prefix = parts[..end].join("/");
        if has_git(&vault.join(&prefix)) {
            return Route { prefix, inner: parts[end..].join("/") };
        }
    }
    Route { prefix: String::new(), inner: rel }
}

/// Agrupa rutas por repositorio, manteniendo el orden de aparición de los repositorios.
pub fn group(vault: &Path, paths: &[String]) -> Vec<(String, Vec<String>)> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for p in paths {
        let r = route(vault, p);
        match groups.iter_mut().find(|(prefix, _)| *prefix == r.prefix) {
            Some((_, list)) => list.push(r.inner),
            None => groups.push((r.prefix, vec![r.inner])),
        }
    }
    groups
}

/// Vuelve a poner la carpeta del repositorio delante de una ruta relativa a él.
pub fn prefixed(prefix: &str, inner: &str) -> String {
    if prefix.is_empty() {
        inner.to_string()
    } else if inner.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{inner}")
    }
}

/// Repositorios de subcarpetas de la bóveda (relativos a ella, sin la raíz), en orden.
pub fn nested_repos(vault: &Path) -> Vec<String> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (Instant, Vec<String>)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(guard) = cache.lock()
        && let Some((at, list)) = guard.get(vault)
            && at.elapsed() < CACHE_TTL {
                return list.clone();
            }
    let mut found = Vec::new();
    scan(vault, "", 0, &mut found);
    found.sort();
    if let Ok(mut guard) = cache.lock() {
        guard.insert(vault.to_path_buf(), (Instant::now(), found.clone()));
    }
    found
}

fn scan(dir: &Path, rel: &str, depth: usize, found: &mut Vec<String>) {
    if depth >= MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || SKIP_DIRS.iter().any(|d| d.eq_ignore_ascii_case(&name)) {
            continue;
        }
        let child_rel = prefixed(rel, &name);
        let path = entry.path();
        if has_git(&path) {
            found.push(child_rel.clone());
        }
        scan(&path, &child_rel, depth + 1, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enruta_al_repositorio_mas_cercano() {
        let base = std::env::temp_dir().join(format!("synapse-routing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("a/b/.git")).unwrap();
        std::fs::create_dir_all(base.join("a/b/c/d/.git")).unwrap();
        std::fs::create_dir_all(base.join("x/node_modules/y/.git")).unwrap();
        std::fs::write(base.join("e"), "").unwrap();

        assert_eq!(route(&base, "a/b/n.md"), Route { prefix: "a/b".into(), inner: "n.md".into() });
        assert_eq!(route(&base, "a/b/c/d/z/n.md"), Route { prefix: "a/b/c/d".into(), inner: "z/n.md".into() });
        assert_eq!(route(&base, "a/b"), Route { prefix: "a/b".into(), inner: "".into() });
        assert_eq!(route(&base, "a/n.md"), Route { prefix: "".into(), inner: "a/n.md".into() });
        assert_eq!(nested_repos(&base), vec!["a/b".to_string(), "a/b/c/d".to_string()]);
        assert_eq!(
            group(&base, &["a/b/1".into(), "e".into(), "a/b/2".into()]),
            vec![("a/b".to_string(), vec!["1".to_string(), "2".to_string()]), ("".to_string(), vec!["e".to_string()])]
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Lo que usan el MCP y los comandos: el trait enruta al repositorio más cercano.
    #[test]
    fn estado_e_historial_en_repositorio_anidado() {
        use crate::domain::version::version_control::VersionControl;
        use crate::infrastructure::repository::git::version_control_impl::VersionControlImpl;
        use std::process::Command;

        let base = std::env::temp_dir().join(format!("synapse-multirepo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let sub = base.join("proyecto");
        std::fs::create_dir_all(&sub).unwrap();
        let git = |dir: &Path, args: &[&str]| {
            let ok = Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok, "git {args:?}");
        };
        std::fs::write(base.join("raiz.md"), "r").unwrap();
        std::fs::write(base.join(".gitignore"), "proyecto/\n").unwrap();
        git(&base, &["init", "-q", "-b", "main"]);
        git(&base, &["add", "."]);
        git(&base, &["commit", "-qm", "raiz"]);
        std::fs::write(sub.join("nota.md"), "uno").unwrap();
        git(&sub, &["init", "-q", "-b", "feature-x"]);
        git(&sub, &["add", "."]);
        git(&sub, &["commit", "-qm", "inicio"]);
        std::fs::write(sub.join("nota.md"), "dos").unwrap();

        let vc = VersionControlImpl::new();
        let st = VersionControl::get_vault_status(&vc, &base).unwrap();
        assert!(st.is_repo);
        assert_eq!(st.branch.as_deref(), Some("main"));
        assert!(st.statuses.contains_key("proyecto/nota.md"), "{:?}", st.statuses);
        let sub_info = st.repos.iter().find(|r| r.path == "proyecto").unwrap();
        assert_eq!(sub_info.branch.as_deref(), Some("feature-x"));
        assert_eq!(sub_info.changes, 1);

        let log = VersionControl::get_file_log(&vc, &base, "proyecto/nota.md", Some(5)).unwrap();
        assert_eq!(log.path, "proyecto/nota.md");
        assert_eq!(log.commits.len(), 1);
        assert_eq!(log.commits[0].message.trim(), "inicio");

        let rev = VersionControl::get_file_revision(&vc, &base, "proyecto/nota.md", "HEAD").unwrap();
        assert_eq!(rev.content.as_deref(), Some("uno"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
