use crate::application::use_cases::mcp_use_cases::McpUseCases;
use crate::domain::models::mcp::*;
use crate::domain::repositories::note_repository::NoteRepository;
use crate::domain::services::full_text_index::FullTextIndex;
use crate::domain::services::git_service::GitPort;
use crate::infrastructure::repositories::file_note_repository::FileNoteRepository;
use crate::infrastructure::services::git_service::GitService;
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Servidor MCP local que se comunica a través de entrada/salida estándar (stdio) con JSON-RPC 2.0.
pub struct McpStdioServer<R: NoteRepository, I: FullTextIndex + ?Sized, G: GitPort> {
    use_cases: McpUseCases<R, I, G>,
}

impl<R: NoteRepository, I: FullTextIndex + ?Sized, G: GitPort> McpStdioServer<R, I, G> {
    pub fn new(use_cases: McpUseCases<R, I, G>) -> Self {
        Self { use_cases }
    }

    /// Ejecuta el bucle de eventos stdio hasta EOF
    pub fn run_loop<Rdr: BufRead, Wtr: Write>(&self, mut reader: Rdr, mut writer: Wtr) -> io::Result<()> {
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if let Some(response_json) = self.handle_line(trimmed) {
                    writer.write_all(response_json.as_bytes())?;
                    writer.write_all(b"\n")?;
                    writer.flush()?;
                }
            }
            line.clear();
        }
        Ok(())
    }

    /// Procesa una sola línea de entrada JSON-RPC y genera la respuesta correspondiente si aplica
    pub fn handle_line(&self, line: &str) -> Option<String> {
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(err) => {
                let err_resp = JsonRpcErrorResponse::new(None, -32700, format!("Parse error: {}", err));
                return serde_json::to_string(&err_resp).ok();
            }
        };

        // Si id es None, puede ser una notificación (e.g. notifications/initialized)
        let is_notification = request.id.is_none();
        let id_val = request.id.unwrap_or(Value::Null);

        let response = match request.method.as_str() {
            "initialize" => {
                let result = self.use_cases.initialize();
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "notifications/initialized" => {
                // Notificación del cliente confirmando inicialización, no requiere respuesta
                None
            }
            "tools/list" => {
                let result = self.use_cases.list_tools();
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "tools/call" => {
                let params = request.params.unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned();
                let result = self.use_cases.call_tool(tool_name, args);
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "resources/list" => {
                let result = self.use_cases.list_resources();
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "resources/read" => {
                let params = request.params.unwrap_or(Value::Null);
                let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                match self.use_cases.read_resource(uri) {
                    Ok(result) => Some(JsonRpcResponse::new(id_val, json!(result))),
                    Err(err) => {
                        let err_resp = JsonRpcErrorResponse::new(Some(id_val), -32602, err);
                        return serde_json::to_string(&err_resp).ok();
                    }
                }
            }
            "prompts/list" => {
                let result = self.use_cases.list_prompts();
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "prompts/get" => {
                let params = request.params.unwrap_or(Value::Null);
                let prompt_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned();
                match self.use_cases.get_prompt(prompt_name, args) {
                    Ok(result) => Some(JsonRpcResponse::new(id_val, json!(result))),
                    Err(err) => {
                        let err_resp = JsonRpcErrorResponse::new(Some(id_val), -32602, err);
                        return serde_json::to_string(&err_resp).ok();
                    }
                }
            }
            "ping" => Some(JsonRpcResponse::new(id_val, json!({}))),
            other => {
                if is_notification {
                    None
                } else {
                    let err_resp = JsonRpcErrorResponse::new(
                        Some(id_val),
                        -32601,
                        format!("Método no encontrado: '{}'", other),
                    );
                    return serde_json::to_string(&err_resp).ok();
                }
            }
        };

        response.and_then(|r| serde_json::to_string(&r).ok())
    }
}

pub type McpStatusCallback = Arc<dyn Fn(McpRuntimeStatus) + Send + Sync>;

/// Proveedor dinámico para obtener el workspace actual y componentes del AppState
pub trait McpContextProvider: Send + Sync {
    fn get_context(&self) -> (Option<PathBuf>, Option<Arc<dyn FullTextIndex>>);
}

/// Servidor MCP en segundo plano (HTTP Streamable / JSON-RPC) que corre en el mismo proceso de Synapse
pub struct McpBackgroundRuntime {
    is_running: Arc<AtomicBool>,
    port: Arc<AtomicUsize>,
    requests_handled: Arc<AtomicUsize>,
    last_error: Arc<Mutex<Option<String>>>,
    worker_handle: Mutex<Option<thread::JoinHandle<()>>>,
    provider: Arc<dyn McpContextProvider>,
    on_status_change: Arc<Mutex<Option<McpStatusCallback>>>,
}

impl McpBackgroundRuntime {
    pub const DEFAULT_PORT: u16 = 27182;

    pub fn new(provider: Arc<dyn McpContextProvider>) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            port: Arc::new(AtomicUsize::new(Self::DEFAULT_PORT as usize)),
            requests_handled: Arc::new(AtomicUsize::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            worker_handle: Mutex::new(None),
            provider,
            on_status_change: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_on_status_change(&self, cb: McpStatusCallback) {
        if let Ok(mut lock) = self.on_status_change.lock() {
            *lock = Some(cb);
        }
    }

    fn notify_status_change(&self) {
        let status = self.get_status();
        if let Ok(lock) = self.on_status_change.lock() {
            if let Some(ref cb) = *lock {
                cb(status);
            }
        }
    }

    pub fn get_status(&self) -> McpRuntimeStatus {
        let running = self.is_running.load(Ordering::SeqCst);
        let port = self.port.load(Ordering::SeqCst) as u16;
        let endpoint = format!("http://127.0.0.1:{}/mcp", port);
        let (vault_opt, _) = self.provider.get_context();
        let active_vault = vault_opt.map(|p| p.to_string_lossy().to_string());
        let requests_handled = self.requests_handled.load(Ordering::SeqCst);
        let last_error = self.last_error.lock().ok().and_then(|g| g.clone());

        McpRuntimeStatus {
            running,
            port,
            endpoint,
            active_vault,
            requests_handled,
            last_error,
        }
    }

    pub fn start(&self) -> Result<McpRuntimeStatus, String> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(self.get_status());
        }

        let desired_port = Self::DEFAULT_PORT;
        let listener = match TcpListener::bind(format!("127.0.0.1:{}", desired_port)) {
            Ok(l) => {
                self.port.store(desired_port as usize, Ordering::SeqCst);
                l
            }
            Err(e) => {
                // Si el puerto por defecto está ocupado, intentar con puerto libre dinámico
                match TcpListener::bind("127.0.0.1:0") {
                    Ok(l) => {
                        let actual_port = l.local_addr().map_err(|e| e.to_string())?.port();
                        self.port.store(actual_port as usize, Ordering::SeqCst);
                        l
                    }
                    Err(e2) => {
                        let err_msg = format!("Fallo al enlazar socket TCP: {} (intento libre: {})", e, e2);
                        if let Ok(mut lock) = self.last_error.lock() {
                            *lock = Some(err_msg.clone());
                        }
                        return Err(err_msg);
                    }
                }
            }
        };

        // Configurar timeout no bloqueante en listener para poder evaluar is_running regularmente
        if let Err(e) = listener.set_nonblocking(true) {
            log::warn!("No se pudo configurar modo no bloqueante en TcpListener MCP: {}", e);
        }

        self.is_running.store(true, Ordering::SeqCst);
        if let Ok(mut lock) = self.last_error.lock() {
            *lock = None;
        }

        let is_running_flag = Arc::clone(&self.is_running);
        let port_num = self.port.load(Ordering::SeqCst) as u16;
        let requests_counter = Arc::clone(&self.requests_handled);
        let provider = Arc::clone(&self.provider);
        let last_error_arc = Arc::clone(&self.last_error);
        let on_status_change = Arc::clone(&self.on_status_change);

        let handle = thread::Builder::new()
            .name("mcp-inprocess-server".to_string())
            .spawn(move || {
                // Timeout para no bloquear indefinidamente el thread en accept()
                let _ = listener.set_ttl(100);

                while is_running_flag.load(Ordering::SeqCst) {
                    // Timeout en incoming o poll
                    match listener.accept() {
                        Ok((stream, _peer)) => {
                            if !is_running_flag.load(Ordering::SeqCst) {
                                break;
                            }
                            requests_counter.fetch_add(1, Ordering::SeqCst);
                            let provider_cloned = Arc::clone(&provider);
                            let provider_for_status = Arc::clone(&provider);
                            let reqs = Arc::clone(&requests_counter);
                            let is_run = Arc::clone(&is_running_flag);
                            let cb_lock = Arc::clone(&on_status_change);
                            let err_lock = Arc::clone(&last_error_arc);

                            // Procesar la conexión en hilo separado o inline
                            thread::spawn(move || {
                                if let Err(e) = Self::handle_http_client(stream, provider_cloned) {
                                    if let Ok(mut lock) = err_lock.lock() {
                                        *lock = Some(e.to_string());
                                    }
                                }
                                if let Ok(lock) = cb_lock.lock() {
                                    if let Some(ref cb) = *lock {
                                        let (vault_opt, _) = provider_for_status.get_context();
                                        cb(McpRuntimeStatus {
                                            running: is_run.load(Ordering::SeqCst),
                                            port: port_num,
                                            endpoint: format!("http://127.0.0.1:{}/mcp", port_num),
                                            active_vault: vault_opt.map(|p| p.to_string_lossy().to_string()),
                                            requests_handled: reqs.load(Ordering::SeqCst),
                                            last_error: err_lock.lock().ok().and_then(|g| g.clone()),
                                        });
                                    }
                                }
                            });
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                            thread::sleep(Duration::from_millis(50));
                        }
                        Err(_) => {
                            if !is_running_flag.load(Ordering::SeqCst) {
                                break;
                            }
                            thread::sleep(Duration::from_millis(50));
                        }
                    }
                }
            })
            .map_err(|e| format!("Error al iniciar hilo MCP: {}", e))?;

        if let Ok(mut worker_lock) = self.worker_handle.lock() {
            *worker_lock = Some(handle);
        }

        self.notify_status_change();
        Ok(self.get_status())
    }

    pub fn stop(&self) -> Result<McpRuntimeStatus, String> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(self.get_status());
        }

        self.is_running.store(false, Ordering::SeqCst);

        // Despertar al TcpListener intentando una conexión rápida local para salir del accept()
        let port = self.port.load(Ordering::SeqCst);
        let _ = TcpStream::connect(format!("127.0.0.1:{}", port));

        if let Ok(mut lock) = self.worker_handle.lock() {
            if let Some(handle) = lock.take() {
                let _ = handle.join();
            }
        }

        self.notify_status_change();
        Ok(self.get_status())
    }

    pub fn toggle(&self) -> Result<McpRuntimeStatus, String> {
        if self.is_running.load(Ordering::SeqCst) {
            self.stop()
        } else {
            self.start()
        }
    }

    fn handle_http_client(mut stream: TcpStream, provider: Arc<dyn McpContextProvider>) -> io::Result<()> {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

        let mut reader = BufReader::new(&mut stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line)? == 0 {
            return Ok(());
        }

        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(());
        }
        let method = parts[0];
        let path = parts[1];

        // Leer cabeceras HTTP
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
                content_length = val.trim().parse().unwrap_or(0);
            }
        }

        // Manejo de CORS Preflight
        if method == "OPTIONS" {
            let response = "HTTP/1.1 204 No Content\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
                Access-Control-Allow-Headers: Content-Type, Authorization, Accept\r\n\
                Access-Control-Max-Age: 86400\r\n\
                Content-Length: 0\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
            return Ok(());
        }

        // Endpoint GET /health o info
        if method == "GET" {
            let (vault, _) = provider.get_context();
            let vault_str = vault.map(|v| v.to_string_lossy().to_string()).unwrap_or_else(|| "none".to_string());
            let body = json!({
                "service": "synapse-mcp-server",
                "status": "healthy",
                "active_vault": vault_str
            })
            .to_string();

            let header = format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: application/json\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Content-Length: {}\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body.as_bytes());
            let _ = stream.flush();
            return Ok(());
        }

        // Endpoint POST /mcp o POST /
        if method == "POST" && (path == "/mcp" || path == "/") {
            let mut body_bytes = vec![0u8; content_length];
            if content_length > 0 {
                reader.read_exact(&mut body_bytes)?;
            }
            let body_str = String::from_utf8_lossy(&body_bytes);

            let (vault_opt, fts_opt) = provider.get_context();
            let active_vault = vault_opt.unwrap_or_else(|| PathBuf::from("."));
            let note_repo = FileNoteRepository::new();
            let git_service = Some(Arc::new(GitService));

            let use_cases = McpUseCases::new(active_vault, note_repo, fts_opt, git_service);
            let stdio_server = McpStdioServer::new(use_cases);

            let response_body = if let Some(resp) = stdio_server.handle_line(body_str.trim()) {
                resp
            } else {
                // Notificación o vacío
                "".to_string()
            };

            let header = format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: application/json\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Content-Length: {}\r\n\r\n",
                response_body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(response_body.as_bytes());
            let _ = stream.flush();
            return Ok(());
        }

        // 404 para cualquier otra ruta
        let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(not_found.as_bytes());
        let _ = stream.flush();
        Ok(())
    }
}
