use super::port_registry;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::search::full_text_index::FullTextIndex;
use crate::domain::version::version_control::VersionControl;
use crate::infrastructure::entrypoint::mcp::dto::*;
use crate::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use crate::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use crate::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Servidor MCP local que se comunica a través de entrada/salida estándar (stdio) con JSON-RPC 2.0.
pub struct McpStdioServer<R: NoteRepository, I: FullTextIndex + ?Sized, G: VersionControl> {
    tool_handler: McpToolHandler<R, I, G>,
}

impl<R: NoteRepository, I: FullTextIndex + ?Sized, G: VersionControl> McpStdioServer<R, I, G> {
    pub fn new(tool_handler: McpToolHandler<R, I, G>) -> Self {
        Self { tool_handler }
    }

    /// Ejecuta el bucle de eventos stdio hasta EOF
    pub fn run_loop<Rdr: BufRead, Wtr: Write>(
        &self,
        mut reader: Rdr,
        mut writer: Wtr,
    ) -> io::Result<()> {
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty()
                && let Some(response_json) = self.handle_line(trimmed) {
                    writer.write_all(response_json.as_bytes())?;
                    writer.write_all(b"\n")?;
                    writer.flush()?;
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
                let err_resp =
                    JsonRpcErrorResponse::new(None, -32700, format!("Parse error: {}", err));
                return serde_json::to_string(&err_resp).ok();
            }
        };

        // Si id es None, puede ser una notificación (e.g. notifications/initialized)
        let is_notification = request.id.is_none();
        let id_val = request.id.unwrap_or(Value::Null);

        let response = match request.method.as_str() {
            "initialize" => {
                let result = self.tool_handler.initialize();
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "notifications/initialized" => {
                // Notificación del cliente confirmando inicialización, no requiere respuesta
                None
            }
            "tools/list" => {
                let result = self.tool_handler.list_tools();
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "tools/call" => {
                let params = request.params.unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned();
                let result = self.tool_handler.call_tool(tool_name, args);
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "resources/list" => {
                let result = self.tool_handler.list_resources();
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "resources/read" => {
                let params = request.params.unwrap_or(Value::Null);
                let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                match self.tool_handler.read_resource(uri) {
                    Ok(result) => Some(JsonRpcResponse::new(id_val, json!(result))),
                    Err(err) => {
                        let err_resp = JsonRpcErrorResponse::new(Some(id_val), -32602, err);
                        return serde_json::to_string(&err_resp).ok();
                    }
                }
            }
            "prompts/list" => {
                let result = self.tool_handler.list_prompts();
                Some(JsonRpcResponse::new(id_val, json!(result)))
            }
            "prompts/get" => {
                let params = request.params.unwrap_or(Value::Null);
                let prompt_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned();
                match self.tool_handler.get_prompt(prompt_name, args) {
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
    /// Quién dibuja los diagramas como imagen, si hay una ventana de Synapse.
    fn diagram_renderer(
        &self,
    ) -> Option<Arc<dyn crate::application::service::diagram_renderer::DiagramRenderer>> {
        None
    }
    /// Otras bóvedas consultables y a quién pedir permiso (la ventana).
    fn vault_access(
        &self,
    ) -> (
        Option<Arc<dyn crate::application::service::vault_catalog::VaultCatalog>>,
        Option<Arc<dyn crate::application::service::vault_catalog::UserPrompter>>,
    ) {
        (None, None)
    }
    /// Las conversaciones de agentes de la ventana, si la hay.
    fn conversation_hub(
        &self,
    ) -> Option<Arc<dyn crate::application::service::conversation_hub::ConversationHub>> {
        None
    }
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
    current_vault: Arc<Mutex<Option<PathBuf>>>,
}

impl McpBackgroundRuntime {
    pub const DEFAULT_PORT: u16 = port_registry::DEFAULT_MCP_PORT;

    pub fn new(provider: Arc<dyn McpContextProvider>) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            port: Arc::new(AtomicUsize::new(Self::DEFAULT_PORT as usize)),
            requests_handled: Arc::new(AtomicUsize::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            worker_handle: Mutex::new(None),
            provider,
            on_status_change: Arc::new(Mutex::new(None)),
            current_vault: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_on_status_change(&self, cb: McpStatusCallback) {
        if let Ok(mut lock) = self.on_status_change.lock() {
            *lock = Some(cb);
        }
    }

    fn notify_status_change(&self) {
        let status = self.get_status();
        if let Ok(lock) = self.on_status_change.lock()
            && let Some(ref cb) = *lock {
                cb(status);
            }
    }

    pub fn get_status(&self) -> McpRuntimeStatus {
        let running = self.is_running.load(Ordering::SeqCst);
        let (vault_opt, _) = self.provider.get_context();
        let port = if running {
            self.port.load(Ordering::SeqCst) as u16
        } else if let Some(ref v) = vault_opt {
            port_registry::get_vault_port(v).unwrap_or(Self::DEFAULT_PORT)
        } else {
            self.port.load(Ordering::SeqCst) as u16
        };
        let endpoint = format!("http://127.0.0.1:{}/mcp", port);
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

        let (vault_opt, _) = self.provider.get_context();
        let (listener, actual_port) = match vault_opt {
            Some(ref vault_path) => {
                let res = port_registry::allocate_and_bind_port(vault_path, None)?;
                port_registry::record_mcp_started(vault_path, res.1);
                if let Ok(mut lock) = self.current_vault.lock() {
                    *lock = Some(vault_path.clone());
                }
                res
            }
            None => {
                let l = TcpListener::bind(format!("127.0.0.1:{}", Self::DEFAULT_PORT))
                    .or_else(|_| TcpListener::bind("127.0.0.1:0"))
                    .map_err(|e| format!("Fallo al enlazar socket TCP: {}", e))?;
                let p = l.local_addr().map_err(|e| e.to_string())?.port();
                (l, p)
            }
        };

        self.port.store(actual_port as usize, Ordering::SeqCst);

        // Configurar timeout no bloqueante en listener para poder evaluar is_running regularmente
        if let Err(e) = listener.set_nonblocking(true) {
            log::warn!(
                "No se pudo configurar modo no bloqueante en TcpListener MCP: {}",
                e
            );
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
                                if let Err(e) = Self::handle_http_client(stream, provider_cloned)
                                    && let Ok(mut lock) = err_lock.lock() {
                                        *lock = Some(e.to_string());
                                    }
                                if let Ok(lock) = cb_lock.lock()
                                    && let Some(ref cb) = *lock {
                                        let (vault_opt, _) = provider_for_status.get_context();
                                        cb(McpRuntimeStatus {
                                            running: is_run.load(Ordering::SeqCst),
                                            port: port_num,
                                            endpoint: format!("http://127.0.0.1:{}/mcp", port_num),
                                            active_vault: vault_opt
                                                .map(|p| p.to_string_lossy().to_string()),
                                            requests_handled: reqs.load(Ordering::SeqCst),
                                            last_error: err_lock
                                                .lock()
                                                .ok()
                                                .and_then(|g| g.clone()),
                                        });
                                    }
                            });
                        }
                        Err(ref e)
                            if e.kind() == io::ErrorKind::WouldBlock
                                || e.kind() == io::ErrorKind::TimedOut =>
                        {
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

    /// Detiene el servidor MCP y libera el hilo/socket efímero sin deshabilitar
    /// la preferencia del vault en disco (usado para shutdown de ventana o cambio de vault).
    pub fn stop(&self) -> Result<McpRuntimeStatus, String> {
        let (vault_opt, _) = self.provider.get_context();
        let port = self.port.load(Ordering::SeqCst) as u16;

        let target_vault = self
            .current_vault
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .or_else(|| vault_opt.clone());

        if let Some(ref v) = target_vault {
            port_registry::record_mcp_runtime_stopped(v);
        }

        if !self.is_running.load(Ordering::SeqCst) {
            self.notify_status_change();
            return Ok(self.get_status());
        }

        self.is_running.store(false, Ordering::SeqCst);

        // Despertar al TcpListener intentando una conexión rápida local para salir del accept()
        let _ = TcpStream::connect(format!("127.0.0.1:{}", port));

        if let Ok(mut lock) = self.worker_handle.lock()
            && let Some(handle) = lock.take() {
                let _ = handle.join();
            }

        self.notify_status_change();
        Ok(self.get_status())
    }

    /// Desactiva explícitamente el servidor MCP para la bóveda (decisión voluntaria del usuario),
    /// persistiendo `enabled: false` en `.synapse/mcp.json` y deteniendo el socket.
    pub fn disable(&self) -> Result<McpRuntimeStatus, String> {
        let (vault_opt, _) = self.provider.get_context();
        let port = self.port.load(Ordering::SeqCst) as u16;

        let target_vault = self
            .current_vault
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .or_else(|| vault_opt.clone());

        if let Some(ref v) = target_vault {
            port_registry::record_mcp_disabled(v, port);
        }

        self.stop()
    }

    pub fn toggle(&self) -> Result<McpRuntimeStatus, String> {
        if self.is_running.load(Ordering::SeqCst) {
            self.disable()
        } else {
            self.start()
        }
    }

    /// Sincroniza el estado del servidor MCP con la configuración específica de la nueva bóveda
    pub fn sync_for_vault(&self, new_vault: &Path) {
        // `SYNAPSE_START_MCP`: ventana abierta por otra bóveda para conversar con ella
        let should_be_enabled = port_registry::is_vault_enabled(new_vault)
            || std::env::var_os("SYNAPSE_START_MCP").is_some();
        let currently_running = self.is_running.load(Ordering::SeqCst);

        let is_same_vault = if let Ok(guard) = self.current_vault.lock() {
            guard.as_ref().map(|v| v == new_vault).unwrap_or(false)
        } else {
            false
        };

        if should_be_enabled {
            if !currently_running || !is_same_vault {
                if currently_running {
                    let _ = self.stop();
                }
                let _ = self.start();
            }
        } else if currently_running {
            let _ = self.stop();
        } else {
            // Actualizar current_vault y notificar para que el puerto refleje el de la nueva bóveda
            if let Ok(mut lock) = self.current_vault.lock() {
                *lock = Some(new_vault.to_path_buf());
            }
            if let Some(port) = port_registry::get_vault_port(new_vault) {
                self.port.store(port as usize, Ordering::SeqCst);
            }
            self.notify_status_change();
        }
    }

    fn handle_http_client(
        mut stream: TcpStream,
        provider: Arc<dyn McpContextProvider>,
    ) -> io::Result<()> {
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
        let mut peer_token: Option<String> = None;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
                content_length = val.trim().parse().unwrap_or(0);
            }
            if let Some((k, v)) = line.split_once(':')
                && k.trim().eq_ignore_ascii_case("x-synapse-token")
            {
                peer_token = Some(v.trim().to_string());
            }
        }

        // Canal entre instancias de Synapse: solo con el token de esta instancia
        if method == "POST" && path == "/peer" {
            let mut body = vec![0u8; content_length];
            if content_length > 0 {
                reader.read_exact(&mut body)?;
            }
            let authorized = peer_token.as_deref() == Some(super::port_registry::instance_token());
            let (status, payload) = if !authorized {
                ("403 Forbidden", json!({"error": "Token inválido"}))
            } else {
                let request: serde_json::Value =
                    serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
                use crate::infrastructure::integration::service::acp::debug_log;
                let ping = request.get("action").and_then(serde_json::Value::as_str) == Some("ping");
                if !ping {
                    debug_log::log_value("peer-in", json!({"request": request}));
                }
                let answer = match provider.conversation_hub().map(|hub| hub.peer(request)) {
                    Some(Ok(v)) => ("200 OK", json!({"result": v})),
                    Some(Err(e)) => ("200 OK", json!({"error": e})),
                    None => ("200 OK", json!({"error": "Sin ventana de Synapse"})),
                };
                if !ping {
                    debug_log::log_value("peer-answer", answer.1.clone());
                }
                answer
            };
            let body = payload.to_string();
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body.as_bytes());
            let _ = stream.flush();
            return Ok(());
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
            let vault_str = vault
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string());
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
        // `?conv=<id>`: la conversación de agente que usa este MCP (para hablar con las demás)
        let (path, query) = path.split_once('?').unwrap_or((path, ""));
        let caller = query
            .split('&')
            .find_map(|kv| kv.strip_prefix("conv="))
            .filter(|c| {
                !c.is_empty() && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            })
            .map(str::to_string);
        if method == "POST" && (path == "/mcp" || path == "/") {
            let mut body_bytes = vec![0u8; content_length];
            if content_length > 0 {
                reader.read_exact(&mut body_bytes)?;
            }
            let body_str = String::from_utf8_lossy(&body_bytes);

            let (vault_opt, fts_opt) = provider.get_context();
            let active_vault = vault_opt.unwrap_or_else(|| PathBuf::from("."));
            let note_repo = NoteRepositoryImpl::new();
            let git_service = Some(Arc::new(VersionControlImpl));

            let tool_handler = McpToolHandler::new(active_vault.clone(), note_repo, fts_opt, git_service, std::sync::Arc::new(crate::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl), std::sync::Arc::new(crate::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl))
                .with_diagram_renderer(provider.diagram_renderer())
                .with_conversations(provider.conversation_hub(), caller.clone());
            let (catalog, prompter) = provider.vault_access();
            let tool_handler = tool_handler.with_vaults(catalog, prompter);
            let stdio_server = McpStdioServer::new(tool_handler);

            let started_at = now_unix_ms();
            let response_body = if let Some(resp) = stdio_server.handle_line(body_str.trim()) {
                resp
            } else {
                // Notificación o vacío
                "".to_string()
            };
            let ended_at = now_unix_ms();

            if let Some(conv_id) = caller.as_deref() {
                record_agent_mcp_call(
                    &active_vault,
                    conv_id,
                    started_at,
                    ended_at,
                    body_str.trim(),
                    &response_body,
                );
            }

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

static MCP_CALL_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Registra el request y response de una llamada de un agente al MCP de Synapse en el sidecar
/// de telemetría de su conversación (`<bóveda>/.synapse/agent-sessions/telemetry/<conv>.json`).
fn record_agent_mcp_call(
    vault: &Path,
    conv_id: &str,
    started_at: i64,
    ended_at: i64,
    request_raw: &str,
    response_raw: &str,
) {
    let Ok(request) = serde_json::from_str::<Value>(request_raw) else {
        return;
    };
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if method.is_empty() || method == "ping" || method == "notifications/initialized" {
        return;
    }
    let tool_name = (method == "tools/call").then(|| {
        request
            .get("params")
            .and_then(|p| p.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string)
    }).flatten();
    let jsonrpc_id = request.get("id").cloned();
    let response = if response_raw.is_empty() {
        Value::Null
    } else {
        serde_json::from_str::<Value>(response_raw).unwrap_or_else(|_| Value::String(response_raw.to_string()))
    };
    let is_error = response.get("error").is_some()
        || response
            .get("result")
            .and_then(|r| r.get("isError"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let seq = MCP_CALL_SEQ.fetch_add(1, Ordering::Relaxed);
    let call = crate::domain::agent::McpCallTelemetry {
        id: format!("mcp-{started_at}-{seq}"),
        started_at,
        ended_at,
        duration_ms: ended_at.saturating_sub(started_at) as u64,
        method,
        tool_name,
        jsonrpc_id,
        request,
        response,
        is_error,
    };
    use crate::domain::agent::AgentTelemetryRepository;
    let repo = crate::infrastructure::repository::file::agent_telemetry_repository_impl::AgentTelemetryRepositoryImpl::default();
    if let Err(e) = repo.record_mcp_call(&vault.to_string_lossy(), conv_id, call) {
        log::debug!("No se pudo registrar telemetría MCP para {conv_id}: {e}");
    }
}
