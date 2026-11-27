//! Ejecutable de Git: el configurado en `<config>/config.jsonc` (`git.path`) o el del PATH.
//!
//! Todas las operaciones que siguen necesitando el `git` real (commit, checkout, push, clone, LFS…)
//! crean su proceso con [`git_command`], así que respetan la ruta elegida por el usuario.

use serde::Serialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::RwLock;

static CONFIGURED: RwLock<Option<PathBuf>> = RwLock::new(None);

const HEADER: &str = "// Configuración global de Synapse: vale para todas las bóvedas.\n";

/// `Command` para el Git real (ruta configurada o `git` del PATH).
pub fn git_command() -> Command {
    let program = CONFIGURED
        .read()
        .ok()
        .and_then(|p| p.clone())
        .unwrap_or_else(|| PathBuf::from("git"));
    // Solo Windows lo modifica
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        // Sin ventana de consola parpadeando por cada comando
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    /// Ruta guardada en la configuración (si hay)
    pub configured_path: Option<String>,
    /// Versión del Git que se está usando, o `None` si no se encontró
    pub version: Option<String>,
}

/// `git --version` del ejecutable dado.
pub fn version_of(program: &Path) -> Option<String> {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    text.starts_with("git version").then_some(text)
}

/// Ruta configurada, si hay.
pub fn configured_path() -> Option<PathBuf> {
    CONFIGURED.read().ok().and_then(|p| p.clone())
}

pub fn info() -> GitInfo {
    let configured = CONFIGURED.read().ok().and_then(|p| p.clone());
    let program = configured.clone().unwrap_or_else(|| PathBuf::from("git"));
    GitInfo {
        configured_path: configured.map(|p| p.to_string_lossy().into_owned()),
        version: version_of(&program),
    }
}

fn config_path(config_dir: &Path) -> PathBuf {
    config_dir.join("config.jsonc")
}

pub(crate) fn read_config(config_dir: &Path) -> Value {
    std::fs::read_to_string(config_path(config_dir))
        .ok()
        .and_then(|raw| serde_json_lenient::from_str::<Value>(&raw).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

pub(crate) fn write_config(config_dir: &Path, config: &Value) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| format!("No se pudo crear {}: {e}", config_dir.display()))?;
    let body = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(config_path(config_dir), format!("{HEADER}{body}\n"))
        .map_err(|e| format!("No se pudo guardar {}: {e}", config_path(config_dir).display()))
}

/// Carga `git.path` de la configuración al arrancar.
pub fn load(config_dir: &Path) {
    let path = read_config(config_dir)
        .pointer("/git/path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from);
    if let Ok(mut c) = CONFIGURED.write() {
        *c = path;
    }
}

/// Guarda (o borra, con `None`) la ruta de Git. Comprueba antes que es un Git que funciona.
pub fn save(config_dir: &Path, path: Option<&str>) -> Result<GitInfo, String> {
    let path = path.map(str::trim).filter(|p| !p.is_empty()).map(PathBuf::from);
    if let Some(p) = &path
        && version_of(p).is_none() {
            return Err(format!("{} no es un ejecutable de Git que funcione (`git --version` falló)", p.display()));
        }
    let mut config = read_config(config_dir);
    let root = config.as_object_mut().expect("read_config devuelve un objeto");
    match &path {
        Some(p) => {
            let git = root.entry("git").or_insert_with(|| json!({}));
            if !git.is_object() {
                *git = json!({});
            }
            git["path"] = json!(p.to_string_lossy());
        }
        None => {
            if let Some(git) = root.get_mut("git").and_then(Value::as_object_mut) {
                git.remove("path");
                if git.is_empty() {
                    root.remove("git");
                }
            }
        }
    }
    write_config(config_dir, &config)?;
    if let Ok(mut c) = CONFIGURED.write() {
        *c = path;
    }
    Ok(info())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarda_y_borra_la_ruta_sin_tocar_el_resto_de_la_configuracion() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.jsonc"), "{ // comentario\n \"appearance\": { \"theme_dirs\": [\"~/t\"] } }").unwrap();
        let git = which_git();
        let info = save(dir.path(), Some(&git)).unwrap();
        assert_eq!(info.configured_path.as_deref(), Some(git.as_str()));
        assert!(info.version.unwrap().starts_with("git version"));
        let v: Value = serde_json_lenient::from_str(&std::fs::read_to_string(dir.path().join("config.jsonc")).unwrap()).unwrap();
        assert_eq!(v.pointer("/git/path").and_then(Value::as_str), Some(git.as_str()));
        assert_eq!(v.pointer("/appearance/theme_dirs/0").and_then(Value::as_str), Some("~/t"));

        load(dir.path());
        assert_eq!(info_path(), Some(git.clone()));

        save(dir.path(), None).unwrap();
        let v: Value = serde_json_lenient::from_str(&std::fs::read_to_string(dir.path().join("config.jsonc")).unwrap()).unwrap();
        assert!(v.get("git").is_none());
        assert_eq!(info_path(), None);
    }

    #[test]
    fn rechaza_un_ejecutable_que_no_es_git() {
        let dir = tempfile::tempdir().unwrap();
        assert!(save(dir.path(), Some("/bin/true")).unwrap_err().contains("no es un ejecutable de Git"));
        assert!(!dir.path().join("config.jsonc").exists());
    }

    fn which_git() -> String {
        let out = Command::new("sh").args(["-c", "command -v git"]).output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn info_path() -> Option<String> {
        CONFIGURED.read().unwrap().as_ref().map(|p| p.to_string_lossy().into_owned())
    }
}
