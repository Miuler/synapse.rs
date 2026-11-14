use crate::application::service::peer_instances::PeerInstances;
use crate::infrastructure::entrypoint::mcp::port_registry::{self, PeerInstance};
use crate::infrastructure::entrypoint::mcp::stdio_http_bridge::post_with;
use serde_json::{Value, json};
use std::time::Duration;

/// Habla con las otras instancias por HTTP local (`/peer`), con el token de cada una del registro.
#[derive(Clone, Default)]
pub struct PeerInstancesImpl;

fn call(peer: &PeerInstance, request: &Value, timeout: Duration) -> Result<Value, String> {
    let body = request.to_string();
    let url = format!("http://127.0.0.1:{}/peer", peer.port);
    let raw = post_with(&url, &body, &[("X-Synapse-Token", &peer.token)], timeout)?;
    let reply: Value =
        serde_json::from_str(&raw).map_err(|e| format!("Respuesta inválida: {e}"))?;
    match reply.get("error").and_then(Value::as_str) {
        Some(e) => Err(e.to_string()),
        None => Ok(reply.get("result").cloned().unwrap_or(Value::Null)),
    }
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
        let peers = port_registry::live_peers();
        let wanted = vault.trim();
        let peer = peers
            .iter()
            .find(|p| p.vault_path == wanted || p.vault_name.eq_ignore_ascii_case(wanted))
            .ok_or_else(|| {
                format!(
                    "La bóveda «{wanted}» no está abierta en otra ventana de Synapse. Abiertas: {}",
                    if peers.is_empty() {
                        "ninguna".to_string()
                    } else {
                        peers
                            .iter()
                            .map(|p| p.vault_name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                )
            })?;
        call(peer, &request, Duration::from_secs(15))
    }
}
