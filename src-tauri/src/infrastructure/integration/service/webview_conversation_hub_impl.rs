use crate::application::service::conversation_hub::ConversationHub;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

/// La ventana responde enseguida (encola si hace falta): no se espera al otro agente.
const TIMEOUT: Duration = Duration::from_secs(10);

type Pending = Arc<Mutex<HashMap<String, mpsc::Sender<Result<Value, String>>>>>;

/// Pregunta a la ventana (evento `mcp://conversations`) y espera su respuesta, que llega con el
/// comando `mcp_conversations_reply`.
#[derive(Clone)]
pub struct WebviewConversationHubImpl {
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
}

impl WebviewConversationHubImpl {
    pub fn new(app_handle: Arc<Mutex<Option<tauri::AppHandle>>>) -> Self {
        Self {
            app_handle,
            pending: Arc::default(),
            next_id: Arc::default(),
        }
    }

    /// Respuesta de la ventana a una petición; las que ya no se esperan se ignoran.
    pub fn resolve(&self, request_id: &str, result: Result<Value, String>) {
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut p| p.remove(request_id));
        if let Some(tx) = sender {
            let _ = tx.send(result);
        }
    }

    fn ask(&self, payload: Value) -> Result<Value, String> {
        let handle = self
            .app_handle
            .lock()
            .ok()
            .and_then(|h| h.clone())
            .ok_or("La ventana de Synapse no está disponible")?;
        let id = format!("c{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::channel();
        self.pending
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id.clone(), tx);
        let mut payload = payload;
        payload["requestId"] = json!(id);
        if let Err(e) = handle.emit("mcp://conversations", payload) {
            self.resolve(&id, Err(e.to_string()));
        }
        let result = rx
            .recv_timeout(TIMEOUT)
            .unwrap_or_else(|_| Err("La ventana de Synapse no respondió a tiempo".into()));
        if let Ok(mut p) = self.pending.lock() {
            p.remove(&id);
        }
        result
    }
}

impl ConversationHub for WebviewConversationHubImpl {
    fn list(&self, caller: Option<&str>) -> Result<Value, String> {
        self.ask(json!({"action": "list", "caller": caller}))
    }

    fn send(
        &self,
        caller: Option<&str>,
        to: &str,
        message: &str,
        expects_reply: bool,
    ) -> Result<Value, String> {
        self.ask(json!({
            "action": "send",
            "caller": caller,
            "to": to,
            "message": message,
            "expectsReply": expects_reply,
        }))
    }
}
