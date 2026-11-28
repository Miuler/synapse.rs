use crate::application::dto::issue::source_draft::{DraftChange, SourceDraft};
use crate::application::service::config_window::ConfigWindow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tauri::Emitter;

/// Tiempo máximo que se espera a que la ventana aplique un cambio.
const APPLY_TIMEOUT: Duration = Duration::from_secs(5);

type Reply = Result<SourceDraft, String>;

/// La ventana avisa de la configuración abierta (comando `mcp_config_draft`). Para cambiarla se le
/// envía `mcp://config-change` y responde con `mcp_config_changed`.
#[derive(Default)]
pub struct WebviewConfigWindowImpl {
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    draft: Mutex<Option<SourceDraft>>,
    pending: Mutex<HashMap<String, mpsc::Sender<Reply>>>,
    next_id: AtomicU64,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl WebviewConfigWindowImpl {
    pub fn new(app_handle: Arc<Mutex<Option<tauri::AppHandle>>>) -> Self {
        Self {
            app_handle,
            ..Self::default()
        }
    }

    /// La configuración abierta ahora, o `None` al cerrarla.
    pub fn set(&self, draft: Option<SourceDraft>) {
        *lock(&self.draft) = draft;
    }

    /// Respuesta de la ventana a un cambio; las que ya no se esperan se ignoran.
    pub fn changed(&self, request_id: &str, reply: Reply) {
        let Some(sender) = lock(&self.pending).remove(request_id) else {
            return;
        };
        if let Ok(draft) = &reply {
            self.set(Some(draft.clone()));
        }
        let _ = sender.send(reply);
    }
}

impl ConfigWindow for WebviewConfigWindowImpl {
    fn current(&self) -> Option<SourceDraft> {
        lock(&self.draft).clone()
    }

    fn propose(&self, change: &DraftChange) -> Reply {
        let handle = lock(&self.app_handle)
            .clone()
            .ok_or("La ventana de Synapse no está disponible")?;
        let id = format!("c{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (sender, receiver) = mpsc::channel();
        lock(&self.pending).insert(id.clone(), sender);
        let sent = handle.emit(
            "mcp://config-change",
            serde_json::json!({"requestId": id, "change": change}),
        );
        let reply = match sent {
            Ok(()) => receiver
                .recv_timeout(APPLY_TIMEOUT)
                .unwrap_or_else(|_| Err("La ventana de Synapse no respondió a tiempo".into())),
            Err(e) => Err(e.to_string()),
        };
        lock(&self.pending).remove(&id);
        reply
    }
}
