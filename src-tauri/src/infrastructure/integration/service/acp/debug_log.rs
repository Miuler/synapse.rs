//! Registro de depuración de ACP: cada línea JSON-RPC que entra y sale de los agentes, sin filtrar,
//! en `<configuración>/acp-debug.jsonl`. Se activa en Ajustes; apagado no cuesta nada.

use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);
static FILE_LOCK: Mutex<()> = Mutex::new(());

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Archivo del registro.
pub fn path() -> PathBuf {
    crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl::resolve_config_dir()
        .join("acp-debug.jsonl")
}

/// Anota un mensaje: `dir` es `in` (del agente) o `out` (hacia el agente).
pub(super) fn log(dir: &str, line: &str) {
    if !is_enabled() {
        return;
    }
    let msg = serde_json::from_str::<Value>(line.trim()).unwrap_or_else(|_| Value::String(line.to_string()));
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let entry = json!({"t": ms, "pid": std::process::id(), "dir": dir, "msg": msg});
    let Ok(_guard) = FILE_LOCK.lock() else { return };
    let path = path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{entry}");
    }
}
