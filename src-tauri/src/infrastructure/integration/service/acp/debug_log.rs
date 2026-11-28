//! Registro de depuración de ACP: cada línea JSON-RPC que entra y sale de los agentes (solo se
//! ocultan las cabeceras y el entorno de sus servidores MCP, que llevan secretos), y los pedidos del canal `/peer` entre instancias de Synapse, en `<configuración>/acp-debug.jsonl`.
//! Se activa en Ajustes; apagado no cuesta nada.

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
    log_value(dir, msg);
}

/// Anota un mensaje ya interpretado (p. ej. un pedido `/peer`: `peer-in`, `peer-out`, `peer-reply`).
/// Los `ping` (saber qué instancias están abiertas) no se anotan: son ruido.
pub fn log_value(dir: &str, mut msg: Value) {
    if !is_enabled() || msg.get("action").and_then(Value::as_str) == Some("ping") {
        return;
    }
    redact(&mut msg);
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

/// Oculta los valores de las cabeceras y del entorno de cualquier lista `mcpServers`.
fn redact(msg: &mut Value) {
    match msg {
        Value::Object(map) => {
            for (key, value) in map {
                match value {
                    Value::Array(servers) if key == "mcpServers" => {
                        servers.iter_mut().for_each(redact_server);
                    }
                    other => redact(other),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        _ => {}
    }
}

fn redact_server(server: &mut Value) {
    for field in ["headers", "env"] {
        let Some(entries) = server.get_mut(field).and_then(Value::as_array_mut) else {
            continue;
        };
        for value in entries.iter_mut().filter_map(|entry| entry.get_mut("value")) {
            *value = json!("•••");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_se_anotan_los_secretos_de_los_servidores_mcp() {
        let mut msg = json!({"method": "session/new", "params": {"cwd": "/b", "mcpServers": [
            {"type": "http", "url": "http://127.0.0.1/mcp",
             "headers": [{"name": "x-synapse-session", "value": "secreto-1"}]},
            {"command": "synapsers", "env": [{"name": "SYNAPSE_MCP_SESSION", "value": "secreto-2"}]},
        ]}});
        redact(&mut msg);
        let text = msg.to_string();
        assert!(!text.contains("secreto"), "{text}");
        assert!(text.contains("x-synapse-session") && text.contains("/b"), "{text}");
    }
}
