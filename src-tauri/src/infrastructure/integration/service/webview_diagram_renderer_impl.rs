use crate::application::service::diagram_renderer::{DiagramRenderer, RenderedImage};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

/// Tiempo máximo que se espera a que la ventana dibuje el diagrama.
const RENDER_TIMEOUT: Duration = Duration::from_secs(30);

type Pending = Arc<Mutex<HashMap<String, mpsc::Sender<Result<RenderedImage, String>>>>>;

/// Pide a la ventana de Synapse (evento `mcp://render-diagram`) que dibuje el diagrama y espera la
/// respuesta, que llega con el comando `mcp_diagram_rendered`.
#[derive(Clone)]
pub struct WebviewDiagramRendererImpl {
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
}

impl WebviewDiagramRendererImpl {
    pub fn new(app_handle: Arc<Mutex<Option<tauri::AppHandle>>>) -> Self {
        Self {
            app_handle,
            pending: Arc::default(),
            next_id: Arc::default(),
        }
    }

    /// Respuesta de la ventana a una petición; las que ya no se esperan se ignoran.
    pub fn resolve(&self, request_id: &str, result: Result<RenderedImage, String>) {
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut p| p.remove(request_id));
        if let Some(tx) = sender {
            let _ = tx.send(result);
        }
    }
}

impl DiagramRenderer for WebviewDiagramRendererImpl {
    fn render(&self, relative_path: &str, content: Option<&str>) -> Result<RenderedImage, String> {
        let handle = self
            .app_handle
            .lock()
            .ok()
            .and_then(|h| h.clone())
            .ok_or("La ventana de Synapse no está disponible para dibujar el diagrama")?;
        let id = format!("r{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (tx, rx) = mpsc::channel();
        self.pending
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id.clone(), tx);
        let sent = handle.emit(
            "mcp://render-diagram",
            serde_json::json!({"requestId": id, "relativePath": relative_path, "content": content}),
        );
        if let Err(e) = sent {
            self.resolve(&id, Err(e.to_string()));
        }
        let result = rx.recv_timeout(RENDER_TIMEOUT).unwrap_or_else(|_| {
            Err("La ventana de Synapse no respondió a tiempo al dibujar el diagrama".into())
        });
        if let Ok(mut p) = self.pending.lock() {
            p.remove(&id);
        }
        result
    }
}
