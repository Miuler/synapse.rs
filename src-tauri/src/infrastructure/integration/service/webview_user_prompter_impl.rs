use crate::application::service::vault_catalog::UserPrompter;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

/// Tiempo para que el usuario responda; sin respuesta, se toma como «no».
const TIMEOUT: Duration = Duration::from_secs(120);

type Pending = Arc<Mutex<HashMap<String, mpsc::Sender<bool>>>>;

/// Pregunta en la ventana de Synapse (evento `mcp://confirm`); responde el comando `mcp_confirm_reply`.
#[derive(Clone)]
pub struct WebviewUserPrompterImpl {
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
}

impl WebviewUserPrompterImpl {
    pub fn new(app_handle: Arc<Mutex<Option<tauri::AppHandle>>>) -> Self {
        Self {
            app_handle,
            pending: Arc::default(),
            next_id: Arc::default(),
        }
    }

    pub fn resolve(&self, request_id: &str, allowed: bool) {
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut p| p.remove(request_id));
        if let Some(tx) = sender {
            let _ = tx.send(allowed);
        }
    }
}

impl UserPrompter for WebviewUserPrompterImpl {
    fn confirm(&self, title: &str, message: &str) -> Result<bool, String> {
        let handle = self
            .app_handle
            .lock()
            .ok()
            .and_then(|h| h.clone())
            .ok_or("La ventana de Synapse no está disponible para pedir permiso")?;
        let id = format!("q{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::channel();
        self.pending
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id.clone(), tx);
        handle
            .emit(
                "mcp://confirm",
                serde_json::json!({"requestId": id, "title": title, "message": message}),
            )
            .map_err(|e| e.to_string())?;
        let answer = rx.recv_timeout(TIMEOUT);
        if let Ok(mut p) = self.pending.lock() {
            p.remove(&id);
        }
        answer.map_err(|_| "El usuario no respondió a tiempo".to_string())
    }
}
