//! Conexión JSON-RPC 2.0 delimitada por líneas con un agente ACP.

use super::agent_runner_impl::{AgentEventSink, config_of};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, oneshot};

type Pending = HashMap<u64, oneshot::Sender<Result<Value, String>>>;

pub(super) struct Connection {
    writer: Mutex<Box<dyn AsyncWrite + Send + Unpin>>,
    next_id: AtomicU64,
    pending: Arc<Mutex<Pending>>,
    /// Peticiones de permiso del agente sin responder: id expuesto → id JSON-RPC original.
    permissions: Arc<Mutex<HashMap<String, Value>>>,
    /// Solicitudes de elicitación del agente sin responder: id expuesto → id JSON-RPC original.
    elicitations: Arc<Mutex<HashMap<String, Value>>>,
}

impl Connection {
    /// Arranca la lectura de `reader`. `session_of` da la sesión para los avisos sin `sessionId`.
    pub(super) fn spawn(
        reader: impl AsyncRead + Send + Unpin + 'static,
        writer: impl AsyncWrite + Send + Unpin + 'static,
        sink: AgentEventSink,
        on_close: impl FnOnce() + Send + 'static,
    ) -> Arc<Self> {
        let conn = Arc::new(Self {
            writer: Mutex::new(Box::new(writer)),
            next_id: AtomicU64::new(1),
            pending: Arc::default(),
            permissions: Arc::default(),
            elicitations: Arc::default(),
        });
        let reader_conn = conn.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                super::debug_log::log("in", &line);
                let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
                    tracing::debug!("ACP: línea que no es JSON: {line}");
                    continue;
                };
                reader_conn.dispatch(msg, &sink).await;
            }
            // El agente cerró: falla lo que esperaba respuesta
            for (_, tx) in reader_conn.pending.lock().await.drain() {
                let _ = tx.send(Err("El agente terminó inesperadamente".into()));
            }
            on_close();
        });
        conn
    }

    async fn dispatch(&self, msg: Value, sink: &AgentEventSink) {
        let method = msg.get("method").and_then(Value::as_str);
        let id = msg.get("id").cloned();
        match (method, id) {
            // Respuesta a una petición nuestra
            (None, Some(id)) => {
                let Some(id) = id.as_u64() else { return };
                if let Some(tx) = self.pending.lock().await.remove(&id) {
                    let result = match msg.get("error") {
                        Some(err) => Err(error_message(err)),
                        None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
                    };
                    let _ = tx.send(result);
                }
            }
            // Petición del agente
            (Some(method), Some(id)) => {
                let params = msg.get("params").cloned().unwrap_or(Value::Null);
                if method == "session/request_permission" {
                    let request_id = id.to_string();
                    self.permissions.lock().await.insert(request_id.clone(), id);
                    sink(
                        "acp://permission-request",
                        permission_event(&params, &request_id),
                    );
                } else if method == "elicitation/create" {
                    let request_id = id.to_string();
                    self.elicitations.lock().await.insert(request_id.clone(), id);
                    sink(
                        "acp://elicitation-request",
                        elicitation_event(&params, &request_id),
                    );
                } else {
                    // No ofrecemos sistema de archivos ni terminal al agente
                    let _ = self
                        .write(json!({"jsonrpc": "2.0", "id": id,
                            "error": {"code": -32601, "message": format!("Método no soportado: {method}")}}))
                        .await;
                }
            }
            // Aviso del agente
            (Some("session/update"), None) => {
                if let Some(params) = msg.get("params") {
                    for (event, payload) in update_events(params) {
                        sink(event, payload);
                    }
                }
            }
            _ => {}
        }
    }

    async fn write(&self, msg: Value) -> Result<(), String> {
        let mut line = serde_json::to_vec(&msg).map_err(|e| e.to_string())?;
        if super::debug_log::is_enabled() {
            super::debug_log::log("out", &String::from_utf8_lossy(&line));
        }
        line.push(b'\n');
        let mut w = self.writer.lock().await;
        w.write_all(&line)
            .await
            .map_err(|e| format!("No se pudo escribir al agente: {e}"))?;
        w.flush()
            .await
            .map_err(|e| format!("No se pudo escribir al agente: {e}"))
    }

    pub(super) async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        if let Err(e) = self
            .write(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .await
        {
            self.pending.lock().await.remove(&id);
            return Err(e);
        }
        rx.await
            .map_err(|_| "El agente terminó inesperadamente".to_string())?
    }

    pub(super) async fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        self.write(json!({"jsonrpc": "2.0", "method": method, "params": params}))
            .await
    }

    /// Responde una petición de permiso pendiente con el resultado ACP.
    pub(super) async fn answer_permission(
        &self,
        request_id: &str,
        outcome: Value,
    ) -> Result<(), String> {
        let id = self
            .permissions
            .lock()
            .await
            .remove(request_id)
            .ok_or("La petición de permiso ya no está pendiente")?;
        self.write(json!({"jsonrpc": "2.0", "id": id, "result": {"outcome": outcome}}))
            .await
    }

    /// Responde una solicitud de elicitación pendiente (`elicitation/create`) con el resultado ACP.
    pub(super) async fn answer_elicitation(
        &self,
        request_id: &str,
        result: Value,
    ) -> Result<(), String> {
        let id = self
            .elicitations
            .lock()
            .await
            .remove(request_id)
            .ok_or("La solicitud de elicitación ya no está pendiente")?;
        self.write(json!({"jsonrpc": "2.0", "id": id, "result": result}))
            .await
    }
}

fn error_message(err: &Value) -> String {
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Error del agente");
    match err.get("data") {
        Some(Value::String(d)) => format!("{message}: {d}"),
        Some(Value::Object(o)) => match o
            .get("details")
            .or(o.get("message"))
            .and_then(Value::as_str)
        {
            Some(d) => format!("{message}: {d}"),
            None => message.to_string(),
        },
        _ => message.to_string(),
    }
}

fn permission_event(params: &Value, request_id: &str) -> Value {
    let tool = params.get("toolCall").cloned().unwrap_or(Value::Null);
    let options: Vec<Value> = params
        .get("options")
        .and_then(Value::as_array)
        .map(|opts| {
            opts.iter()
                .map(|o| {
                    json!({
                        "id": o.get("optionId").cloned().unwrap_or(Value::Null),
                        "name": o.get("name").cloned().unwrap_or(Value::Null),
                        "kind": o.get("kind").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    json!({
        "sessionId": params.get("sessionId").cloned().unwrap_or(Value::Null),
        "requestId": request_id,
        "title": tool.get("title").and_then(Value::as_str).unwrap_or("El agente pide permiso"),
        "toolCallId": tool.get("toolCallId").cloned().unwrap_or(Value::Null),
        "options": options,
    })
}

fn elicitation_event(params: &Value, request_id: &str) -> Value {
    let mode = params
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or_else(|| if params.get("url").is_some() { "url" } else { "form" });
    json!({
        "sessionId": params.get("sessionId").cloned().unwrap_or(Value::Null),
        "requestId": request_id,
        "mode": mode,
        "message": params
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("El agente solicita información"),
        "requestedSchema": params.get("requestedSchema").cloned().unwrap_or(Value::Null),
        "url": params.get("url").cloned().unwrap_or(Value::Null),
        "elicitationId": params.get("elicitationId").cloned().unwrap_or(Value::Null),
    })
}

/// Traduce un `session/update` a los eventos que consume la interfaz.
fn update_events(params: &Value) -> Vec<(&'static str, Value)> {
    let session_id = params.get("sessionId").cloned().unwrap_or(Value::Null);
    let Some(update) = params.get("update") else {
        return vec![];
    };
    let kind = update
        .get("sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or("");
    let text_of = |u: &Value| -> Option<String> {
        let c = u.get("content")?;
        (c.get("type")?.as_str()? == "text").then(|| c.get("text")?.as_str().map(str::to_string))?
    };
    match kind {
        "agent_message_chunk" | "agent_thought_chunk" | "user_message_chunk" => {
            let Some(text) = text_of(update) else {
                return vec![];
            };
            let kind = match kind {
                "agent_message_chunk" => "message",
                "agent_thought_chunk" => "thought",
                _ => "user",
            };
            vec![(
                "acp://stream",
                json!({"sessionId": session_id, "kind": kind, "text": text}),
            )]
        }
        "tool_call" | "tool_call_update" => vec![(
            "acp://tool-call",
            json!({
                "sessionId": session_id,
                "toolCallId": update.get("toolCallId").cloned().unwrap_or(Value::Null),
                "title": update.get("title").cloned().unwrap_or(Value::Null),
                "kind": update.get("kind").cloned().unwrap_or(Value::Null),
                "status": update.get("status").cloned().unwrap_or(Value::Null),
                "locations": update.get("locations").cloned().unwrap_or(Value::Null),
                "content": tool_content(update.get("content")),
                "rawInput": update.get("rawInput").cloned().unwrap_or(Value::Null),
                "rawOutput": update.get("rawOutput").cloned().unwrap_or(Value::Null),
            }),
        )],
        "plan" => vec![(
            "acp://plan",
            json!({"sessionId": session_id, "entries": update.get("entries").cloned().unwrap_or(json!([]))}),
        )],
        "available_commands_update" => vec![(
            "acp://commands",
            json!({
                "sessionId": session_id,
                "commands": update.get("availableCommands").cloned().unwrap_or(json!([])),
            }),
        )],
        "usage_update" => vec![(
            "acp://usage",
            json!({
                "sessionId": session_id,
                "used": update.get("used").cloned().unwrap_or(Value::Null),
                "size": update.get("size").cloned().unwrap_or(Value::Null),
                "cost": update.get("cost").cloned().unwrap_or(Value::Null),
                "quota": super::agent_quota_impl::claude_quota(update),
            }),
        )],
        "current_mode_update" => vec![(
            "acp://mode",
            json!({"sessionId": session_id, "modeId": update.get("currentModeId").cloned().unwrap_or(Value::Null)}),
        )],
        "session_info_update" => match update.get("title").and_then(Value::as_str) {
            Some(title) => vec![(
                "acp://session-info",
                json!({"sessionId": session_id, "title": title}),
            )],
            None => vec![],
        },
        "config_option_update" => vec![(
            "acp://config",
            json!({"sessionId": session_id, "config": config_of(update)}),
        )],
        _ => vec![],
    }
}

/// Contenido de una herramienta: diffs y texto, en una forma simple para la interfaz.
fn tool_content(content: Option<&Value>) -> Value {
    let Some(items) = content.and_then(Value::as_array) else {
        return Value::Null;
    };
    Value::Array(
        items
            .iter()
            .filter_map(|c| match c.get("type").and_then(Value::as_str)? {
                "diff" => Some(json!({
                    "type": "diff",
                    "path": c.get("path").cloned().unwrap_or(Value::Null),
                    "oldText": c.get("oldText").cloned().unwrap_or(Value::Null),
                    "newText": c.get("newText").cloned().unwrap_or(Value::Null),
                })),
                "content" => {
                    let inner = c.get("content")?;
                    (inner.get("type")?.as_str()? == "text")
                        .then(|| json!({"type": "text", "text": inner.get("text").cloned().unwrap_or(Value::Null)}))
                }
                _ => None,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traduce_plan_diff_y_uso() {
        let plan = update_events(
            &json!({"sessionId": "s", "update": {"sessionUpdate": "plan",
            "entries": [{"content": "Leer", "priority": "high", "status": "pending"}]}}),
        );
        assert_eq!(plan[0].0, "acp://plan");
        assert_eq!(plan[0].1["entries"][0]["content"], "Leer");

        let tool = update_events(
            &json!({"sessionId": "s", "update": {"sessionUpdate": "tool_call",
            "toolCallId": "t1", "title": "Editar", "locations": [{"path": "/b/n.md"}],
            "content": [{"type": "diff", "path": "/b/n.md", "oldText": "a", "newText": "b"}]}}),
        );
        assert_eq!(tool[0].1["content"][0]["newText"], "b");
        assert_eq!(tool[0].1["locations"][0]["path"], "/b/n.md");

        let usage = update_events(
            &json!({"sessionId": "s", "update": {"sessionUpdate": "usage_update", "used": 10, "size": 100}}),
        );
        assert_eq!(usage[0].0, "acp://usage");
        assert!(usage[0].1["quota"].is_null());

        let info = update_events(
            &json!({"sessionId": "s", "update": {"sessionUpdate": "session_info_update", "title": "Mi sesión"}}),
        );
        assert_eq!(info[0].0, "acp://session-info");
        assert_eq!(info[0].1["title"], "Mi sesión");
        // Una actualización sin título (solo la fecha) no lo borra
        let untitled = update_events(
            &json!({"sessionId": "s", "update": {"sessionUpdate": "session_info_update", "updatedAt": "2026-01-01T00:00:00Z"}}),
        );
        assert!(untitled.is_empty());
    }

    #[test]
    fn traduce_elicitation_form_y_url() {
        let form = elicitation_event(
            &json!({
                "sessionId": "ses-1",
                "mode": "form",
                "message": "Elige entorno",
                "requestedSchema": {
                    "type": "object",
                    "properties": { "env": { "type": "string", "enum": ["dev", "prod"] } },
                    "required": ["env"]
                }
            }),
            "\"el-1\"",
        );
        assert_eq!(form["sessionId"], "ses-1");
        assert_eq!(form["requestId"], "\"el-1\"");
        assert_eq!(form["mode"], "form");
        assert_eq!(form["message"], "Elige entorno");
        assert_eq!(form["requestedSchema"]["properties"]["env"]["enum"][1], "prod");

        let url = elicitation_event(
            &json!({
                "sessionId": "ses-1",
                "mode": "url",
                "message": "Autoriza en el navegador",
                "url": "https://example.com/oauth",
                "elicitationId": "oauth-42"
            }),
            "\"el-2\"",
        );
        assert_eq!(url["mode"], "url");
        assert_eq!(url["url"], "https://example.com/oauth");
        assert_eq!(url["elicitationId"], "oauth-42");
    }
}
