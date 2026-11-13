use super::connection::Connection;
use crate::application::integration::service::AgentRunner;
use crate::domain::agent::{
    AgentCapabilities, AgentLaunch, AgentOption, AgentSession, PermissionDecision, PromptBlock,
    ToolServer,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Destino de los eventos de los agentes (nombre del evento y contenido).
pub type AgentEventSink = Arc<dyn Fn(&str, Value) + Send + Sync>;

struct RunningAgent {
    conn: Arc<Connection>,
    child: Option<Child>,
    capabilities: AgentCapabilities,
    /// Bóveda de la sesión y si el agente tiene el MCP de Synapse (para indicarle cómo leer las notas).
    cwd: String,
    has_synapse_tools: bool,
    /// Instrucciones de sesión pendientes de enviar con el primer mensaje.
    instructions: Option<String>,
}

/// Lanza cada agente como proceso hijo y conversa con él por ACP sobre `stdin`/`stdout`.
#[derive(Clone)]
pub struct AgentRunnerImpl {
    sink: AgentEventSink,
    sessions: Arc<Mutex<HashMap<String, RunningAgent>>>,
}

impl AgentRunnerImpl {
    pub fn new(sink: AgentEventSink) -> Self {
        Self {
            sink,
            sessions: Arc::default(),
        }
    }

    async fn conn(&self, session_id: &str) -> Result<Arc<Connection>, String> {
        Ok(self.running(session_id).await?.0)
    }

    async fn running(
        &self,
        session_id: &str,
    ) -> Result<(Arc<Connection>, AgentCapabilities, Option<String>), String> {
        self.sessions
            .lock()
            .await
            .get(session_id)
            .map(|r| {
                (
                    r.conn.clone(),
                    r.capabilities.clone(),
                    r.has_synapse_tools.then(|| r.cwd.clone()),
                )
            })
            .ok_or_else(|| "La sesión del agente ya no está abierta".to_string())
    }

    /// Hace el saludo ACP y abre la sesión sobre una conexión ya establecida.
    async fn open(
        &self,
        conn: Arc<Connection>,
        child: Option<Child>,
        agent_id: &str,
        cwd: &str,
        tools: Vec<ToolServer>,
        instructions: Option<String>,
        resume: Option<String>,
    ) -> Result<AgentSession, String> {
        let init = conn.request(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {"fs": {"readTextFile": false, "writeTextFile": false}, "terminal": false},
                "clientInfo": {"name": "synapsers", "version": env!("CARGO_PKG_VERSION")},
            }),
        )
        .await
        .map_err(|e| format!("El agente no respondió al iniciar: {e}"))?;
        let capabilities = capabilities_of(&init);
        let mcp_servers = mcp_servers_for(&tools, &capabilities);
        // Retomar: el agente repite el historial con `session/update` antes de responder; la
        // interfaz aún no conoce la sesión y descarta esos eventos (ya tiene su transcripción).
        let loaded = match resume.filter(|_| capabilities.load_session) {
            Some(old) => conn
                .request(
                    "session/load",
                    json!({"sessionId": old, "cwd": cwd, "mcpServers": mcp_servers}),
                )
                .await
                .inspect_err(|e| tracing::warn!("No se pudo retomar la sesión {old}: {e}"))
                .ok()
                .map(|mut r| {
                    r["sessionId"] = json!(old);
                    r
                }),
            None => None,
        };
        let resumed = loaded.is_some();
        let result = match loaded {
            Some(r) => r,
            None => conn
            .request("session/new", json!({"cwd": cwd, "mcpServers": mcp_servers}))
            .await
            .map_err(|e| {
                if e.to_ascii_lowercase().contains("auth") {
                    format!(
                        "El agente pide iniciar sesión ({e}). Inicia sesión desde su CLI en una terminal \
                         (por ejemplo `agy` o `claude`) y vuelve a conectar."
                    )
                } else {
                    format!("El agente no pudo abrir la sesión: {e}")
                }
            })?,
        };
        let id = result
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or("El agente no devolvió un identificador de sesión")?
            .to_string();
        let (modes, current_mode) =
            options_of(result.get("modes"), "availableModes", "currentModeId", "id");
        let (models, current_model) = options_of(
            result.get("models"),
            "availableModels",
            "currentModelId",
            "modelId",
        );
        self.sessions.lock().await.insert(
            id.clone(),
            RunningAgent {
                conn,
                child,
                capabilities: capabilities.clone(),
                cwd: cwd.to_string(),
                has_synapse_tools: tools.iter().any(|t| t.name == SYNAPSE_TOOLS),
                // Una sesión retomada ya recibió las instrucciones
                instructions: instructions.filter(|_| !resumed),
            },
        );
        Ok(AgentSession {
            id,
            agent_id: agent_id.to_string(),
            cwd: cwd.to_string(),
            capabilities,
            modes,
            current_mode,
            models,
            current_model,
            tool_servers: tools.iter().map(|t| t.name.clone()).collect(),
            resumed,
        })
    }

    #[cfg(test)]
    pub(super) async fn open_with(
        &self,
        reader: impl tokio::io::AsyncRead + Send + Unpin + 'static,
        writer: impl tokio::io::AsyncWrite + Send + Unpin + 'static,
        cwd: &str,
    ) -> Result<AgentSession, String> {
        let conn = Connection::spawn(reader, writer, self.sink.clone(), || {});
        self.open(conn, None, "prueba", cwd, vec![], None, None)
            .await
    }
}

impl AgentRunner for AgentRunnerImpl {
    async fn start(
        &self,
        agent_id: &str,
        launch: AgentLaunch,
        cwd: &str,
        tools: Vec<ToolServer>,
        instructions: Option<String>,
        resume: Option<String>,
    ) -> Result<AgentSession, String> {
        let mut child = Command::new(&launch.command)
            .args(&launch.args)
            .envs(&launch.env)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("No se pudo lanzar el agente «{}»: {e}", launch.command))?;
        let stdin = child.stdin.take().ok_or("Sin stdin del agente")?;
        let stdout = child.stdout.take().ok_or("Sin stdout del agente")?;
        if let Some(stderr) = child.stderr.take() {
            let id = agent_id.to_string();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(agent = %id, "{line}");
                }
            });
        }
        let sink = self.sink.clone();
        let sessions = self.sessions.clone();
        let agent = agent_id.to_string();
        // La sesión se conoce después del saludo; el cierre del proceso solo afecta a la suya
        let slot: Arc<std::sync::OnceLock<String>> = Arc::default();
        let closed = slot.clone();
        let conn = Connection::spawn(stdout, stdin, self.sink.clone(), move || {
            if let Some(id) = closed.get().cloned() {
                tokio::spawn(async move {
                    if sessions.lock().await.remove(&id).is_some() {
                        sink(
                            "acp://status",
                            json!({"sessionId": id, "agentId": agent, "status": "stopped"}),
                        );
                    }
                });
            }
        });
        let session = self
            .open(
                conn,
                Some(child),
                agent_id,
                cwd,
                tools,
                instructions,
                resume,
            )
            .await?;
        let _ = slot.set(session.id.clone());
        Ok(session)
    }

    async fn prompt(&self, session_id: &str, blocks: Vec<PromptBlock>) -> Result<String, String> {
        let (conn, caps, vault) = self.running(session_id).await?;
        let instructions = self
            .sessions
            .lock()
            .await
            .get_mut(session_id)
            .and_then(|r| r.instructions.take());
        let mut prompt: Vec<Value> = Vec::new();
        if let Some(text) = instructions {
            prompt.push(json!({"type": "text", "text": text}));
        }
        prompt.extend(blocks.iter().map(|b| acp_block(b, &caps)));
        if let Some(hint) = vault.and_then(|v| synapse_hint(&blocks, &v)) {
            prompt.push(json!({"type": "text", "text": hint}));
        }
        let result = conn
            .request(
                "session/prompt",
                json!({"sessionId": session_id, "prompt": prompt}),
            )
            .await?;
        Ok(result
            .get("stopReason")
            .and_then(Value::as_str)
            .unwrap_or("end_turn")
            .to_string())
    }

    async fn cancel(&self, session_id: &str) -> Result<(), String> {
        self.conn(session_id)
            .await?
            .notify("session/cancel", json!({"sessionId": session_id}))
            .await
    }

    async fn respond_permission(
        &self,
        session_id: &str,
        request_id: &str,
        decision: PermissionDecision,
    ) -> Result<(), String> {
        let outcome = match decision {
            PermissionDecision::Selected { option_id } => {
                json!({"outcome": "selected", "optionId": option_id})
            }
            PermissionDecision::Cancelled => json!({"outcome": "cancelled"}),
        };
        self.conn(session_id)
            .await?
            .answer_permission(request_id, outcome)
            .await
    }

    async fn set_mode(&self, session_id: &str, mode_id: &str) -> Result<(), String> {
        self.conn(session_id)
            .await?
            .request(
                "session/set_mode",
                json!({"sessionId": session_id, "modeId": mode_id}),
            )
            .await
            .map(|_| ())
    }

    async fn set_model(&self, session_id: &str, model_id: &str) -> Result<(), String> {
        self.conn(session_id)
            .await?
            .request(
                "session/set_model",
                json!({"sessionId": session_id, "modelId": model_id}),
            )
            .await
            .map(|_| ())
    }

    async fn stop(&self, session_id: &str) -> Result<(), String> {
        if let Some(mut running) = self.sessions.lock().await.remove(session_id)
            && let Some(child) = running.child.as_mut()
        {
            let _ = child.kill().await;
        }
        Ok(())
    }
}

/// Nombre con que se inyecta el MCP de Synapse en las sesiones.
pub const SYNAPSE_TOOLS: &str = "synapse_instance";

/// Indicación para que el agente lea las notas adjuntas con el MCP de Synapse, que entiende la
/// estructura de las notas y puede leer solo un esquema, una sección o un tramo de líneas.
fn synapse_hint(blocks: &[PromptBlock], vault: &str) -> Option<String> {
    let rel = |p: &str| {
        p.strip_prefix(vault)
            .unwrap_or(p)
            .trim_start_matches('/')
            .to_string()
    };
    let notes: Vec<String> = blocks
        .iter()
        .filter_map(|b| match b {
            PromptBlock::File { path, .. } => Some(format!("`{}`", rel(path))),
            PromptBlock::Selection {
                path,
                start_line,
                end_line,
                ..
            } if *start_line > 0 => {
                Some(format!("`{}` (líneas {start_line}-{end_line})", rel(path)))
            }
            PromptBlock::Selection { path, .. } => Some(format!("`{}`", rel(path))),
            _ => None,
        })
        .collect();
    if notes.is_empty() {
        return None;
    }
    Some(format!(
        "[Contexto de Synapse] Las notas adjuntas están en la bóveda abierta en Synapse: {}. \
         Léelas con la herramienta MCP `read_note` del servidor `{SYNAPSE_TOOLS}` usando `relative_path`; \
         para notas largas pide primero `mode: \"outline_only\"` y luego `mode: \"section\"` o \
         `\"chunk\"` (`start_line`, `line_count`) en lugar de leer el archivo completo. El servidor \
         `{SYNAPSE_TOOLS}` también ofrece búsqueda, enlaces, backlinks y Git sobre la bóveda.",
        notes.join(", ")
    ))
}

/// Capacidades que el agente anuncia en `initialize`.
fn capabilities_of(init: &Value) -> AgentCapabilities {
    let caps = &init["agentCapabilities"];
    let flag = |v: &Value| v.as_bool().unwrap_or(false);
    AgentCapabilities {
        image: flag(&caps["promptCapabilities"]["image"]),
        embedded_context: flag(&caps["promptCapabilities"]["embeddedContext"]),
        tools_over_http: flag(&caps["mcpCapabilities"]["http"]),
        load_session: flag(&caps["loadSession"]),
    }
}

/// Servidores MCP para `session/new`: HTTP si el agente lo admite, si no un proceso por stdio.
fn mcp_servers_for(tools: &[ToolServer], caps: &AgentCapabilities) -> Vec<Value> {
    tools
        .iter()
        .filter_map(|t| match (&t.http_url, &t.process) {
            (Some(url), _) if caps.tools_over_http => {
                Some(json!({"type": "http", "name": t.name, "url": url, "headers": []}))
            }
            (_, Some(p)) => Some(json!({
                "name": t.name,
                "command": p.command,
                "args": p.args,
                "env": p.env.iter().map(|(k, v)| json!({"name": k, "value": v})).collect::<Vec<_>>(),
            })),
            _ => None,
        })
        .collect()
}

/// Lista de opciones (modos o modelos) de la respuesta de `session/new`.
fn options_of(
    block: Option<&Value>,
    list: &str,
    current: &str,
    id_key: &str,
) -> (Vec<AgentOption>, Option<String>) {
    let Some(block) = block else {
        return (vec![], None);
    };
    let options = block[list]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|o| {
                    Some(AgentOption {
                        id: o[id_key].as_str()?.to_string(),
                        name: o["name"].as_str().unwrap_or_default().to_string(),
                        description: o["description"].as_str().map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (options, block[current].as_str().map(str::to_string))
}

fn file_uri(path: &str) -> String {
    format!("file://{}", path.replace(' ', "%20"))
}

/// Traduce una parte del mensaje a un bloque de contenido ACP según lo que acepta el agente.
fn acp_block(block: &PromptBlock, caps: &AgentCapabilities) -> Value {
    match block {
        PromptBlock::Text { text } => json!({"type": "text", "text": text}),
        PromptBlock::File { path, name } => {
            json!({"type": "resource_link", "uri": file_uri(path), "name": name})
        }
        PromptBlock::Selection {
            path,
            text,
            start_line,
            end_line,
        } => {
            // Línea 0: fragmento de la vista de lectura, sin números de línea
            let lines = (*start_line > 0).then(|| (start_line, end_line));
            if caps.embedded_context {
                let uri = match lines {
                    Some((s, e)) => format!("{}#L{s}-L{e}", file_uri(path)),
                    None => file_uri(path),
                };
                json!({"type": "resource", "resource": {"uri": uri, "mimeType": "text/plain", "text": text}})
            } else {
                let place = match lines {
                    Some((s, e)) => format!("{path} (líneas {s}-{e})"),
                    None => format!("{path} (fragmento)"),
                };
                json!({"type": "text", "text": format!("{place}:\n```\n{text}\n```")})
            }
        }
        PromptBlock::Image { mime_type, data } if caps.image => {
            json!({"type": "image", "mimeType": mime_type, "data": data})
        }
        PromptBlock::Image { .. } => {
            json!({"type": "text", "text": "[imagen omitida: el agente no acepta imágenes]"})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex};

    /// Agente ACP falso: responde el saludo, abre la sesión, emite un trozo de texto,
    /// pide permiso y termina el turno cuando se le responde.
    async fn fake_agent(io: tokio::io::DuplexStream) {
        let (r, mut w) = tokio::io::split(io);
        let mut lines = BufReader::new(r).lines();
        let mut prompt_id = None;
        while let Ok(Some(line)) = lines.next_line().await {
            let msg: Value = serde_json::from_str(&line).unwrap();
            let reply = match msg.get("method").and_then(Value::as_str) {
                Some("initialize") => {
                    vec![json!({"jsonrpc":"2.0","id":msg["id"],"result":{"protocolVersion":1}})]
                }
                Some("session/new") => {
                    vec![json!({"jsonrpc":"2.0","id":msg["id"],"result":{"sessionId":"ses-1"}})]
                }
                Some("session/prompt") => {
                    prompt_id = Some(msg["id"].clone());
                    vec![
                        json!({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"ses-1",
                            "update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Hola"}}}}),
                        json!({"jsonrpc":"2.0","id":"perm-7","method":"session/request_permission","params":{
                            "sessionId":"ses-1","toolCall":{"toolCallId":"t1","title":"Editar nota.md"},
                            "options":[{"optionId":"allow","name":"Permitir","kind":"allow_once"}]}}),
                    ]
                }
                None if msg["id"] == "perm-7" => {
                    assert_eq!(msg["result"]["outcome"]["optionId"], "allow");
                    vec![
                        json!({"jsonrpc":"2.0","id":prompt_id.clone().unwrap(),"result":{"stopReason":"end_turn"}}),
                    ]
                }
                _ => vec![],
            };
            for m in reply {
                w.write_all(format!("{m}\n").as_bytes()).await.unwrap();
            }
        }
    }

    #[tokio::test]
    async fn conversa_por_acp_con_streaming_y_permisos() {
        let events: Arc<StdMutex<Vec<(String, Value)>>> = Arc::default();
        let recorded = events.clone();
        let runner = AgentRunnerImpl::new(Arc::new(move |name, payload| {
            recorded.lock().unwrap().push((name.to_string(), payload));
        }));
        let (client, agent) = duplex(64 * 1024);
        tokio::spawn(fake_agent(agent));
        let (r, w) = tokio::io::split(client);
        let session = runner.open_with(r, w, "/boveda").await.unwrap();
        assert_eq!(session.id, "ses-1");

        let prompt = {
            let runner = runner.clone();
            tokio::spawn(async move {
                runner
                    .prompt(
                        "ses-1",
                        vec![PromptBlock::Text {
                            text: "hola".into(),
                        }],
                    )
                    .await
            })
        };
        // Esperar a que llegue la petición de permiso y aprobarla
        let request_id = loop {
            let found = events.lock().unwrap().iter().find_map(|(n, p)| {
                (n == "acp://permission-request")
                    .then(|| p["requestId"].as_str().unwrap().to_string())
            });
            if let Some(id) = found {
                break id;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        };
        runner
            .respond_permission(
                "ses-1",
                &request_id,
                PermissionDecision::Selected {
                    option_id: "allow".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(prompt.await.unwrap().unwrap(), "end_turn");

        let events = events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|(n, p)| n == "acp://stream" && p["text"] == "Hola" && p["kind"] == "message")
        );
        let perm = events
            .iter()
            .find(|(n, _)| n == "acp://permission-request")
            .unwrap();
        assert_eq!(perm.1["title"], "Editar nota.md");
        assert_eq!(perm.1["options"][0]["id"], "allow");
    }

    fn synapse_tools() -> Vec<ToolServer> {
        vec![ToolServer {
            name: "synapse".into(),
            http_url: Some("http://127.0.0.1:4700/mcp".into()),
            process: Some(AgentLaunch {
                command: "/bin/synapsers".into(),
                args: vec!["--mcp".into()],
                ..Default::default()
            }),
        }]
    }

    #[test]
    fn el_mcp_va_por_http_si_el_agente_lo_admite() {
        let caps = AgentCapabilities {
            tools_over_http: true,
            ..Default::default()
        };
        let servers = mcp_servers_for(&synapse_tools(), &caps);
        assert_eq!(servers[0]["type"], "http");
        assert_eq!(servers[0]["url"], "http://127.0.0.1:4700/mcp");
    }

    #[test]
    fn el_mcp_va_por_stdio_si_no() {
        let servers = mcp_servers_for(&synapse_tools(), &AgentCapabilities::default());
        assert_eq!(servers[0]["command"], "/bin/synapsers");
        assert_eq!(servers[0]["args"][0], "--mcp");
        assert!(servers[0].get("type").is_none());
    }

    #[test]
    fn la_seleccion_se_incrusta_o_se_cita() {
        let sel = PromptBlock::Selection {
            path: "/b/n.md".into(),
            text: "hola".into(),
            start_line: 2,
            end_line: 3,
        };
        let embedded = acp_block(
            &sel,
            &AgentCapabilities {
                embedded_context: true,
                ..Default::default()
            },
        );
        assert_eq!(embedded["resource"]["uri"], "file:///b/n.md#L2-L3");
        let quoted = acp_block(&sel, &AgentCapabilities::default());
        assert!(quoted["text"].as_str().unwrap().contains("líneas 2-3"));
    }

    #[test]
    fn lee_modos_y_modelos() {
        let r = json!({"models": {"currentModelId": "m2", "availableModels": [{"modelId": "m1", "name": "Uno"}, {"modelId": "m2", "name": "Dos"}]}});
        let (models, current) = options_of(
            r.get("models"),
            "availableModels",
            "currentModelId",
            "modelId",
        );
        assert_eq!(models.len(), 2);
        assert_eq!(current.as_deref(), Some("m2"));
    }

    /// Prueba real con un agente instalado: `SYNAPSE_REAL_AGENT=claude-acp|antigravity`,
    /// `SYNAPSE_MCP_BIN=<synapsers>` y `cargo test -- --ignored agente_real`. Descarga Claude a
    /// `XDG_DATA_HOME` si hace falta y usa la cuenta del usuario.
    #[tokio::test]
    #[ignore]
    async fn agente_real() {
        use crate::application::integration::service::AgentInstaller;
        use crate::infrastructure::integration::service::acp::{
            AgentInstallerImpl, registry::RegistryClient,
        };
        let agent = std::env::var("SYNAPSE_REAL_AGENT").unwrap_or_else(|_| "claude-acp".into());
        let launch = AgentInstallerImpl::new(RegistryClient::new())
            .ensure_installed(&agent)
            .await
            .unwrap();
        println!("lanzar: {} {:?}", launch.command, launch.args);
        let vault = tempfile::tempdir().unwrap();
        std::fs::write(
            vault.path().join("receta.md"),
            "# Receta secreta\n\nIngrediente clave: cardamomo.\n",
        )
        .unwrap();
        let cwd = vault.path().to_string_lossy().into_owned();
        let tools: Vec<ToolServer> = std::env::var("SYNAPSE_MCP_BIN")
            .map(|bin| {
                vec![ToolServer {
                    name: SYNAPSE_TOOLS.into(),
                    http_url: None,
                    process: Some(AgentLaunch {
                        command: bin,
                        args: vec!["--mcp".into(), "--vault".into(), cwd.clone()],
                        ..Default::default()
                    }),
                }]
            })
            .unwrap_or_default();
        let events: Arc<StdMutex<Vec<(String, Value)>>> = Arc::default();
        let recorded = events.clone();
        let runner = AgentRunnerImpl::new(Arc::new(move |name, payload| {
            recorded.lock().unwrap().push((name.to_string(), payload));
        }));
        let session = runner
            .start(&agent, launch, &cwd, tools, Some(crate::infrastructure::entrypoint::controller::tauri::agent_controller::session_instructions(true)), None)
            .await
            .unwrap();
        println!("capacidades: {:?}", session.capabilities);
        println!(
            "modos: {:?} (actual {:?})",
            session.modes.iter().map(|m| &m.id).collect::<Vec<_>>(),
            session.current_mode
        );
        println!(
            "modelos: {:?} (actual {:?})",
            session.models.iter().map(|m| &m.id).collect::<Vec<_>>(),
            session.current_model
        );

        // Aprueba automáticamente lo que pida, para no bloquear la prueba
        let approver = {
            let runner = runner.clone();
            let events = events.clone();
            let id = session.id.clone();
            tokio::spawn(async move {
                let mut seen = 0;
                loop {
                    let pending: Vec<(String, String)> = events.lock().unwrap()[seen..]
                        .iter()
                        .filter(|(n, _)| n == "acp://permission-request")
                        .map(|(_, p)| {
                            (
                                p["requestId"].as_str().unwrap().to_string(),
                                p["options"][0]["id"].as_str().unwrap_or("").to_string(),
                            )
                        })
                        .collect();
                    seen = events.lock().unwrap().len();
                    for (req, opt) in pending {
                        println!("permiso aprobado: {req}");
                        let _ = runner
                            .respond_permission(
                                &id,
                                &req,
                                PermissionDecision::Selected { option_id: opt },
                            )
                            .await;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            })
        };
        let stop = runner
            .prompt(
                &session.id,
                vec![
                    PromptBlock::Text {
                        text: std::env::var("SYNAPSE_REAL_PROMPT").unwrap_or_else(|_| {
                            "¿Qué ingrediente clave dice la nota adjunta? Responde en una línea.".into()
                        }),
                    },
                    PromptBlock::File {
                        path: format!("{cwd}/receta.md"),
                        name: "receta.md".into(),
                    },
                ],
            )
            .await;
        approver.abort();
        let events = events.lock().unwrap();
        let reply: String = events
            .iter()
            .filter(|(n, p)| n == "acp://stream" && p["kind"] == "message")
            .map(|(_, p)| p["text"].as_str().unwrap_or(""))
            .collect();
        let kinds: std::collections::BTreeSet<&str> =
            events.iter().map(|(n, _)| n.as_str()).collect();
        let tools_used: Vec<String> = events
            .iter()
            .filter(|(n, _)| n == "acp://tool-call")
            .filter_map(|(_, p)| p["title"].as_str().map(str::to_string))
            .collect();
        println!(
            "fin: {stop:?}\neventos: {kinds:?}\nherramientas: {tools_used:?}\nrespuesta: {reply}"
        );
        for f in std::fs::read_dir(&cwd).unwrap().flatten() {
            println!("archivo en la bóveda: {}", f.file_name().to_string_lossy());
        }
        runner.stop(&session.id).await.unwrap();
        assert!(stop.is_ok());
        assert!(reply.to_lowercase().contains("cardamomo"));
    }

    #[test]
    fn indica_leer_las_notas_con_el_mcp_de_synapse() {
        let blocks = vec![
            PromptBlock::Text {
                text: "hola".into(),
            },
            PromptBlock::File {
                path: "/b/notas/a.md".into(),
                name: "a.md".into(),
            },
            PromptBlock::Selection {
                path: "/b/c.md".into(),
                text: "x".into(),
                start_line: 3,
                end_line: 5,
            },
        ];
        let hint = synapse_hint(&blocks, "/b").unwrap();
        assert!(hint.contains("`notas/a.md`"));
        assert!(hint.contains("`c.md` (líneas 3-5)"));
        assert!(hint.contains("read_note"));
        assert!(synapse_hint(&blocks[..1], "/b").is_none());
    }
}
