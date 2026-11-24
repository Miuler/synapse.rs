//! Rutas que la persona autorizó (diálogo nativo, línea de comandos, bóveda abierta).
//!
//! La interfaz ejecuta contenido ajeno (notas, dibujos, plugins): una ruta que llega desde ella no
//! prueba que nadie la eligiera. Por eso la lista la lleva Rust y solo crece con gestos de la persona.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
pub struct AllowedPaths {
    files: Mutex<HashSet<PathBuf>>,
    vaults: Mutex<HashSet<PathBuf>>,
}

/// Resuelve `.` y `..` sin tocar el disco (no sigue enlaces simbólicos).
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| normalize(path))
}

/// `path` está dentro de `dir`. Un enlace simbólico dentro de `dir` cuenta aunque apunte fuera: lo
/// puso la persona, y con enlaces se organizan muchas bóvedas.
pub fn is_inside(dir: &Path, path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    // `enlace/../x` significa cosas distintas para el disco y para `normalize`: solo vale el destino real
    let has_parent = path.components().any(|c| c == Component::ParentDir);
    if has_parent {
        return path.canonicalize().is_ok_and(|p| p.starts_with(canonical(dir)));
    }
    normalize(path).starts_with(normalize(dir)) || canonical(path).starts_with(canonical(dir))
}

impl AllowedPaths {
    pub fn grant_file(&self, path: &Path) {
        if let Ok(mut files) = self.files.lock() {
            files.insert(canonical(path));
        }
    }

    pub fn has_file(&self, path: &Path) -> bool {
        path.is_absolute() && self.files.lock().is_ok_and(|f| f.contains(&canonical(path)))
    }

    pub fn grant_vault(&self, path: &Path) {
        if let Ok(mut vaults) = self.vaults.lock() {
            vaults.insert(canonical(path));
        }
    }

    pub fn has_vault(&self, path: &Path) -> bool {
        self.vaults.lock().is_ok_and(|v| v.contains(&canonical(path)))
    }

    /// Archivo elegido por la persona o dentro de la bóveda abierta.
    pub fn allows(&self, vault: Option<&Path>, path: &Path) -> bool {
        self.has_file(path) || vault.is_some_and(|v| is_inside(v, path))
    }
}

/// Fondos elegidos con el diálogo: se recuerdan aquí, no en la interfaz, para autorizarlos al arrancar.
pub fn load_wallpapers(config_dir: &Path) -> Vec<PathBuf> {
    std::fs::read_to_string(config_dir.join("wallpapers.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn remember_wallpaper(config_dir: &Path, path: &Path) -> Result<(), String> {
    let mut all = load_wallpapers(config_dir);
    if all.iter().any(|p| p == path) {
        return Ok(());
    }
    all.push(path.to_path_buf());
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(config_dir.join("wallpapers.json"), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_permite_lo_elegido_y_la_boveda() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("boveda");
        std::fs::create_dir_all(vault.join("sub")).unwrap();
        let (inside, outside, other) =
            (vault.join("sub/a.md"), dir.path().join("b.md"), dir.path().join("c.md"));
        for f in [&inside, &outside, &other] {
            std::fs::write(f, "x").unwrap();
        }
        let allowed = AllowedPaths::default();
        assert!(allowed.allows(Some(&vault), &inside));
        assert!(allowed.allows(Some(&vault), &vault.join("sub/nueva.md")));
        assert!(!allowed.allows(Some(&vault), &outside));
        assert!(!allowed.allows(Some(&vault), &vault.join("../b.md")));
        assert!(!allowed.allows(Some(&vault), &vault.join("no/existe/../../../b.md")));
        assert!(!allowed.allows(Some(&vault), &dir.path().join("boveda2/x.md")));
        assert!(!allowed.allows(None, &inside));
        assert!(!allowed.allows(Some(&vault), Path::new("sub/a.md")));
        allowed.grant_file(&outside);
        assert!(allowed.allows(None, &vault.join("../b.md")));
        assert!(!allowed.allows(None, &other));
    }

    #[cfg(unix)]
    #[test]
    fn un_enlace_de_la_boveda_vale_pero_no_sirve_para_subir() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("boveda");
        let ext = dir.path().join("externo/docs");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::create_dir_all(&ext).unwrap();
        std::fs::write(ext.join("a.md"), "x").unwrap();
        std::fs::write(dir.path().join("externo/secreto.md"), "x").unwrap();
        std::os::unix::fs::symlink(&ext, vault.join("enlace")).unwrap();
        let allowed = AllowedPaths::default();
        assert!(allowed.allows(Some(&vault), &vault.join("enlace/a.md")));
        assert!(!allowed.allows(Some(&vault), &vault.join("enlace/../secreto.md")));
        // Tampoco para crear uno nuevo fuera
        assert!(!allowed.allows(Some(&vault), &vault.join("enlace/../nuevo.md")));
    }

    #[test]
    fn recuerda_los_fondos() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_wallpapers(dir.path()).is_empty());
        for _ in 0..2 {
            remember_wallpaper(dir.path(), Path::new("/x/fondo.png")).unwrap();
        }
        assert_eq!(load_wallpapers(dir.path()), [PathBuf::from("/x/fondo.png")]);
    }
}
