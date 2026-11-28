use crate::application::service::peer_instances::PeerInstances;
use crate::infrastructure::entrypoint::mcp::port_registry::{self, PeerInstance};
use crate::infrastructure::entrypoint::mcp::stdio_http_bridge::post_with;
use serde_json::{Value, json};
use std::time::Duration;

/// Habla con las otras instancias por HTTP local (`/peer`), con el token de cada una del registro.
#[derive(Clone, Default)]
pub struct PeerInstancesImpl;

fn call(peer: &PeerInstance, request: &Value, timeout: Duration) -> Result<Value, String> {
    use super::acp::debug_log;
    let ping = request.get("action").and_then(Value::as_str) == Some("ping");
    if !ping {
        debug_log::log_value("peer-out", json!({"to": peer.vault_name, "request": request}));
    }
    let body = request.to_string();
    let url = format!("http://127.0.0.1:{}/peer", peer.port);
    let raw = post_with(&url, &body, &[("X-Synapse-Token", &peer.token)], timeout)?;
    let reply: Value =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid response: {e}"))?;
    if !ping {
        debug_log::log_value("peer-reply", json!({"from": peer.vault_name, "reply": reply}));
    }
    match reply.get("error").and_then(Value::as_str) {
        Some(e) => Err(e.to_string()),
        None => Ok(reply.get("result").cloned().unwrap_or(Value::Null)),
    }
}

/** Ruta de una bóveda reciente por nombre o ruta. */
fn recent_vault(wanted: &str) -> Option<std::path::PathBuf> {
    use crate::application::service::vault_catalog::VaultCatalog;
    super::vault_catalog_impl::VaultCatalogImpl
        .list()
        .into_iter()
        .find(|v| v.name.eq_ignore_ascii_case(wanted) || v.path.to_string_lossy() == wanted)
        .map(|v| v.path)
}

/// Abre la bóveda en otra ventana (otro proceso de Synapse) con su MCP encendido.
fn open_window(path: &std::path::Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe)
        .arg("--vault")
        .arg(path)
        .env("SYNAPSE_START_MCP", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open vault in another window: {e}"))
}

impl PeerInstances for PeerInstancesImpl {
    fn broadcast(&self, request: Value) -> Vec<Value> {
        port_registry::live_peers()
            .into_iter()
            .map(|p| {
                let out = call(&p, &request, Duration::from_secs(4));
                json!({
                    "vault": p.vault_name,
                    "path": p.vault_path,
                    "result": out.as_ref().ok(),
                    "error": out.err(),
                })
            })
            .collect()
    }

    fn send(&self, vault: &str, request: Value) -> Result<Value, String> {
        let wanted = vault.trim();
        let matches =
            |p: &PeerInstance| p.vault_path == wanted || p.vault_name.eq_ignore_ascii_case(wanted);
        let mut peers = port_registry::live_peers();
        // Si la bóveda está entre los recientes pero no abierta, se abre su ventana y se espera su MCP
        if !peers.iter().any(matches)
            && request.get("action").and_then(Value::as_str) == Some("send")
            && let Some(path) = recent_vault(wanted)
        {
            open_window(&path)?;
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            while std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(500));
                peers = port_registry::live_peers();
                if peers.iter().any(matches) {
                    // Que la ventana termine de cargar su panel antes del pedido
                    std::thread::sleep(Duration::from_secs(2));
                    break;
                }
            }
        }
        let peer = peers.iter().find(|p| matches(p)).ok_or_else(|| {
            format!(
                "Vault \"{wanted}\" is not open in another Synapse window. Open: {}",
                if peers.is_empty() {
                    "none".to_string()
                } else {
                    peers
                        .iter()
                        .map(|p| p.vault_name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            )
        })?;
        call(peer, &request, Duration::from_secs(20))
    }
}
