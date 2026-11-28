use crate::application::dto::issue::source_draft::{DraftChange, SourceDraft};
use crate::application::service::config_window::ConfigWindow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tauri::{Emitter, Manager};

/// Tiempo máximo que se espera a que la ventana aplique un cambio.
const APPLY_TIMEOUT: Duration = Duration::from_secs(5);

type Reply = Result<SourceDraft, String>;

/// La ventana avisa de la configuración abierta (comando `mcp_config_draft`). Para cambiarla se le
/// envía `mcp://config-change`, solo a ella, y responde con `mcp_config_changed`.
#[derive(Default)]
pub struct WebviewConfigWindowImpl {
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    /// La configuración abierta y la ventana que la tiene.
    draft: Mutex<Option<(String, SourceDraft)>>,
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

    /// La configuración abierta en `window`, o `None` al cerrarla: una ventana no cierra la de otra.
    pub fn set(&self, window: &str, draft: Option<SourceDraft>) {
        let mut open = lock(&self.draft);
        match draft {
            Some(draft) => *open = Some((window.to_string(), draft)),
            None if open.as_ref().is_some_and(|(owner, _)| owner == window) => *open = None,
            None => {}
        }
    }

    /// Respuesta de la ventana a un cambio; las que ya no se esperan se ignoran.
    pub fn changed(&self, request_id: &str, reply: Reply) {
        let Some(sender) = lock(&self.pending).remove(request_id) else {
            return;
        };
        if let (Ok(draft), Some((_, open))) = (&reply, lock(&self.draft).as_mut()) {
            *open = draft.clone();
        }
        let _ = sender.send(reply);
    }
}

impl ConfigWindow for WebviewConfigWindowImpl {
    fn current(&self) -> Option<SourceDraft> {
        let (owner, draft) = lock(&self.draft).clone()?;
        // Una ventana cerrada no llega a avisar
        match lock(&self.app_handle).as_ref() {
            Some(handle) if handle.get_webview_window(&owner).is_none() => None,
            _ => Some(draft),
        }
    }

    fn propose(&self, change: &DraftChange) -> Reply {
        let owner = lock(&self.draft)
            .as_ref()
            .map(|(owner, _)| owner.clone())
            .ok_or("La configuración se cerró")?;
        let handle = lock(&self.app_handle)
            .clone()
            .ok_or("La ventana de Synapse no está disponible")?;
        let id = format!("c{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (sender, receiver) = mpsc::channel();
        lock(&self.pending).insert(id.clone(), sender);
        let sent = handle.emit_to(
            owner,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_ventana_solo_cierra_su_configuracion() {
        let window = WebviewConfigWindowImpl::default();
        let draft = |scope: &str| SourceDraft {
            scope: scope.into(),
            ..Default::default()
        };
        window.set("main", Some(draft("uno")));
        window.set("otra", None);
        assert_eq!(window.current().unwrap().scope, "uno");
        window.set("otra", Some(draft("dos")));
        window.set("main", None);
        assert_eq!(window.current().unwrap().scope, "dos");
        window.set("otra", None);
        assert_eq!(window.current(), None);
    }
}
