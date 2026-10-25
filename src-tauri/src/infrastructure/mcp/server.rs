use crate::application::use_cases::mcp_use_cases::McpUseCases;
use crate::domain::models::mcp::*;
use crate::domain::repositories::note_repository::NoteRepository;
use crate::domain::services::full_text_index::FullTextIndex;
use crate::domain::services::git_service::GitPort;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

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
