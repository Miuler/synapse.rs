use super::connection::Connection;
use crate::application::integration::service::AgentRunner;
use crate::domain::agent::{
    AgentCapabilities, AgentConfig, AgentLaunch, AgentOption, AgentSession, ElicitationDecision,
    PermissionDecision, PromptBlock,
    ToolServer,
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Destino de los eventos de los agentes (nombre del evento y contenido).
pub type AgentEventSink = Arc<dyn Fn(&str, Value) + Send + Sync>;

/// Tiempo que un proceso sin sesiones sigue vivo, por si se abre otra conversación enseguida.
const IDLE_SHUTDOWN: Duration = Duration::from_secs(30);
/// Conversaciones por proceso: con más, se lanza otro proceso del mismo agente. Así una
/// conversación con un turno largo o un fallo del proceso no arrastra a demasiadas otras.
pub const MAX_SESSIONS_PER_PROCESS: usize = 4;

/// Un proceso de agente ya saludado (`initialize`), compartido por todas sus sesiones.
struct AgentProcess {
    conn: Arc<Connection>,
    child: Option<Child>,
    capabilities: AgentCapabilities,
    sessions: HashSet<String>,
    /// Distingue este proceso de otro posterior con la misma clave.
    generation: u64,
}

/// Una conversación (sesión ACP) sobre un proceso.
struct RunningSession {
    process_key: String,
    conn: Arc<Connection>,
    capabilities: AgentCapabilities,
    /// Bóveda de la sesión y si el agente tiene el MCP de Synapse (para indicarle cómo leer las notas).
    cwd: String,
    has_synapse_tools: bool,
    /// Instrucciones de sesión pendientes de enviar con el primer mensaje.
    instructions: Option<String>,
}

/// Lanza cada agente como proceso hijo y conversa con él por ACP sobre `stdin`/`stdout`. Un mismo
/// proceso atiende todas las conversaciones de ese agente en esa bóveda (`session/new` por cada una).
#[derive(Clone)]
pub struct AgentRunnerImpl {
    sink: AgentEventSink,
    processes: Arc<Mutex<HashMap<String, AgentProcess>>>,
    sessions: Arc<Mutex<HashMap<String, RunningSession>>>,
    generation: Arc<AtomicU64>,
    idle_shutdown: Duration,
    /// Conversaciones por proceso de cada agente (si no está, [`MAX_SESSIONS_PER_PROCESS`]).
    limits: Arc<std::sync::Mutex<HashMap<String, usize>>>,
}

/// Clave del proceso: el mismo agente, en la misma bóveda y lanzado igual.
fn process_key(agent_id: &str, cwd: &str, launch: &AgentLaunch) -> String {
    format!(
        "{agent_id}\u{0}{cwd}\u{0}{}\u{0}{}",
        launch.command,
        launch.args.join("\u{1}")
    )
}

impl AgentRunnerImpl {
    pub fn new(sink: AgentEventSink) -> Self {
        Self {
            sink,
            processes: Arc::default(),
            sessions: Arc::default(),
            generation: Arc::default(),
            idle_shutdown: IDLE_SHUTDOWN,
            limits: Arc::default(),
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

    /// Saludo ACP: devuelve lo que sabe hacer el agente.
    async fn handshake(conn: &Connection) -> Result<AgentCapabilities, String> {
        let init = conn.request(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {
                    "fs": {"readTextFile": false, "writeTextFile": false},
                    "terminal": false,
                    "elicitation": {"form": {}, "url": {}},
                },
                "clientInfo": {"name": "synapsers", "version": env!("CARGO_PKG_VERSION")},
            }),
        )
        .await
        .map_err(|e| format!("El agente no respondió al iniciar: {e}"))?;
        Ok(capabilities_of(&init))
    }

    /// Lanza el proceso del agente, lo saluda y lo registra con `key`.
    async fn spawn_process(
        &self,
        key: &str,
        agent_id: &str,
        launch: &AgentLaunch,
        cwd: &str,
    ) -> Result<(Arc<Connection>, AgentCapabilities), String> {
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
        let generation = self.generation.fetch_add(1, Ordering::Relaxed);
        let conn = Connection::spawn(stdout, stdin, self.sink.clone(), {
            let runner = self.clone();
            let key = key.to_string();
            let agent = agent_id.to_string();
            move || {
                tokio::spawn(async move { runner.process_exited(&key, generation, &agent).await });
            }
        });
        let capabilities = Self::handshake(&conn).await?;
        self.processes.lock().await.insert(
            key.to_string(),
            AgentProcess {
                conn: conn.clone(),
                child: Some(child),
                capabilities: capabilities.clone(),
                sessions: HashSet::new(),
                generation,
            },
        );
        Ok((conn, capabilities))
    }

    /// El proceso terminó: sus conversaciones se cierran y la interfaz lo sabe.
    async fn process_exited(&self, key: &str, generation: u64, agent_id: &str) {
        let mut processes = self.processes.lock().await;
        let Some(process) = processes.get(key).filter(|p| p.generation == generation) else {
            return;
        };
        let ids: Vec<String> = process.sessions.iter().cloned().collect();
        processes.remove(key);
        drop(processes);
        let mut sessions = self.sessions.lock().await;
        for id in ids {
            if sessions.remove(&id).is_some() {
                (self.sink)(
                    "acp://status",
                    json!({"sessionId": id, "agentId": agent_id, "status": "stopped"}),
                );
            }
        }
    }

    /// Abre (o retoma) una sesión sobre un proceso ya saludado.
    #[allow(clippy::too_many_arguments)]
    async fn open_session(
        &self,
        key: &str,
        conn: Arc<Connection>,
        capabilities: AgentCapabilities,
        agent_id: &str,
        cwd: &str,
        tools: Vec<ToolServer>,
        instructions: Option<String>,
        resume: Option<String>,
        fork: bool,
    ) -> Result<AgentSession, String> {
        let mcp_servers = mcp_servers_for(&tools, &capabilities);
        // Retomar: el agente repite el historial con `session/update` antes de responder; la
        // interfaz aún no conoce la sesión y descarta esos eventos (ya tiene su transcripción).
        let (method, supported) = if fork {
            ("session/fork", capabilities.fork_session)
        } else {
            ("session/load", capabilities.load_session)
        };
        let loaded = match resume.filter(|_| supported) {
            Some(old) => conn
                .request(
                    method,
                    json!({"sessionId": old, "cwd": cwd, "mcpServers": mcp_servers}),
                )
                .await
                .inspect_err(|e| tracing::warn!("Falló {method} de la sesión {old}: {e}"))
                .ok()
                .map(|mut r| {
                    if !fork {
                        r["sessionId"] = json!(old);
                    }
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
        let config = config_of(&result);
        if let Some(process) = self.processes.lock().await.get_mut(key) {
            process.sessions.insert(id.clone());
        }
        self.sessions.lock().await.insert(
            id.clone(),
            RunningSession {
                process_key: key.to_string(),
                conn,
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
            config,
            tool_servers: tools.iter().map(|t| t.name.clone()).collect(),
            resumed,
        })
    }

    /// Cierra el proceso si sigue sin sesiones pasado el tiempo de espera.
    fn schedule_idle_shutdown(&self, key: String, generation: u64) {
        let runner = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(runner.idle_shutdown).await;
            let mut processes = runner.processes.lock().await;
            let idle = processes
                .get(&key)
                .is_some_and(|p| p.generation == generation && p.sessions.is_empty());
            if idle && let Some(mut process) = processes.remove(&key) {
                drop(processes);
                if let Some(child) = process.child.as_mut() {
                    let _ = child.kill().await;
                }
            }
        });
    }

    /// Procesos vivos (para las pruebas).
    #[cfg(test)]
    pub(super) async fn process_count(&self) -> usize {
        self.processes.lock().await.len()
    }

    #[cfg(test)]
    pub(super) fn with_idle_shutdown(mut self, idle: Duration) -> Self {
        self.idle_shutdown = idle;
        self
    }

    /// Registra un agente ya conectado como si fuera el proceso de `key` (para las pruebas).
    #[cfg(test)]
    pub(super) async fn register_process(
        &self,
        key: &str,
        reader: impl tokio::io::AsyncRead + Send + Unpin + 'static,
        writer: impl tokio::io::AsyncWrite + Send + Unpin + 'static,
    ) {
        let conn = Connection::spawn(reader, writer, self.sink.clone(), || {});
        let capabilities = Self::handshake(&conn).await.unwrap();
        self.processes.lock().await.insert(
            key.to_string(),
            AgentProcess {
                conn,
                child: None,
                capabilities,
                sessions: HashSet::new(),
                generation: 7,
            },
        );
    }

    /// Conecta con un agente ya establecido (sin proceso) y abre una sesión, para las pruebas.
    #[cfg(test)]
    pub(super) async fn open_with(
        &self,
        reader: impl tokio::io::AsyncRead + Send + Unpin + 'static,
        writer: impl tokio::io::AsyncWrite + Send + Unpin + 'static,
        cwd: &str,
    ) -> Result<AgentSession, String> {
        let conn = Connection::spawn(reader, writer, self.sink.clone(), || {});
        let capabilities = Self::handshake(&conn).await?;
        self.processes.lock().await.insert(
            "prueba".into(),
            AgentProcess {
                conn: conn.clone(),
                child: None,
                capabilities: capabilities.clone(),
                sessions: HashSet::new(),
                generation: u64::MAX,
            },
        );
        self.open_session(
            "prueba",
            conn,
            capabilities,
            "prueba",
            cwd,
            vec![],
            None,
            None,
            false,
        )
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
        fork: bool,
    ) -> Result<AgentSession, String> {
        let base = process_key(agent_id, cwd, &launch);
        // Otra conversación con el mismo agente: se reutiliza un proceso suyo con sitio libre
        let limit = self
            .limits
            .lock()
            .ok()
            .and_then(|l| l.get(agent_id).copied())
            .unwrap_or(MAX_SESSIONS_PER_PROCESS)
            .max(1);
        let existing = self.processes.lock().await.iter().find_map(|(k, p)| {
            (k.starts_with(&base) && p.sessions.len() < limit)
                .then(|| (k.clone(), p.conn.clone(), p.capabilities.clone()))
        });
        let (key, conn, capabilities) = match existing {
            Some(found) => found,
            None => {
                let key = format!("{base}#{}", self.generation.load(Ordering::Relaxed));
                let (conn, caps) = self.spawn_process(&key, agent_id, &launch, cwd).await?;
                (key, conn, caps)
            }
        };
        self.open_session(
            &key,
            conn,
            capabilities,
            agent_id,
            cwd,
            tools,
            instructions,
            resume,
            fork,
        )
        .await
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

    async fn respond_elicitation(
        &self,
        session_id: &str,
        request_id: &str,
        decision: ElicitationDecision,
    ) -> Result<(), String> {
        let result = match decision {
            ElicitationDecision::Accept {
                content: Some(content),
            } => json!({"action": "accept", "content": content}),
            ElicitationDecision::Accept { content: None } => json!({"action": "accept"}),
            ElicitationDecision::Decline => json!({"action": "decline"}),
            ElicitationDecision::Cancel => json!({"action": "cancel"}),
        };
        self.conn(session_id)
            .await?
            .answer_elicitation(request_id, result)
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

    async fn set_option(
        &self,
        session_id: &str,
        option_id: &str,
        value: &str,
    ) -> Result<AgentConfig, String> {
        self.conn(session_id)
            .await?
            .request(
                "session/set_config_option",
                json!({"sessionId": session_id, "configId": option_id, "value": value}),
            )
            .await
            .map(|r| config_of(&r))
    }

    fn set_process_limit(&self, agent_id: &str, limit: usize) {
        if let Ok(mut l) = self.limits.lock() {
            l.insert(agent_id.to_string(), limit.max(1));
        }
    }

    /// Cierra la conversación; el proceso sigue para las demás y se apaga cuando no queda ninguna.
    async fn stop(&self, session_id: &str) -> Result<(), String> {
        let Some(session) = self.sessions.lock().await.remove(session_id) else {
            return Ok(());
        };
        // ACP no tiene un cierre de sesión estándar: se cancela por si estaba trabajando
        let _ = session
            .conn
            .notify("session/cancel", json!({"sessionId": session_id}))
            .await;
        let mut processes = self.processes.lock().await;
        if let Some(process) = processes.get_mut(&session.process_key) {
            process.sessions.remove(session_id);
            if process.sessions.is_empty() {
                self.schedule_idle_shutdown(session.process_key.clone(), process.generation);
            }
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
    // Imágenes y diagramas adjuntos (también un `.drawio.png`): no se leen con read_note
    let visual = |n: &String| {
        let l = n.to_lowercase();
        [".png`", ".jpg`", ".jpeg`", ".gif`", ".webp`", ".svg`", ".drawio`", ".excalidraw`", ".d2`", ".mmd`", ".puml`"]
            .iter()
            .any(|ext| l.contains(ext))
    };
    let visual_hint = if notes.iter().any(visual) {
        " Para ver una imagen o un diagrama adjunto (.png, .svg, .drawio.png, .excalidraw, .d2, .mmd…) usa \
         `render_diagram` con `relative_path`; para el texto y la estructura (contenedores y \
         conexiones) de un draw.io, `extract_note_diagrams`."
            .to_string()
    } else {
        String::new()
    };
    Some(format!(
        "[Contexto de Synapse] Las notas adjuntas están en la bóveda abierta en Synapse: {}. \
         Léelas con la herramienta MCP `read_note` del servidor `{SYNAPSE_TOOLS}` usando `relative_path`; \
         para notas largas pide primero `mode: \"outline_only\"` y luego `mode: \"section\"` o \
         `\"chunk\"` (`start_line`, `line_count`) en lugar de leer el archivo completo.{visual_hint} El servidor \
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
        fork_session: caps["sessionCapabilities"]["fork"].is_object(),
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

/// Modelo y esfuerzo de una respuesta con `configOptions`; si el agente no los anuncia ahí, el modelo
/// sale del bloque `models`. Los agentes que mandan ambos mezclan el esfuerzo en `models`.
pub(super) fn config_of(v: &Value) -> AgentConfig {
    let select = |category: &str, ids: &[&str]| {
        let all = v["configOptions"].as_array()?;
        let found = all
            .iter()
            .find(|o| o["category"] == category)
            .or_else(|| all.iter().find(|o| ids.iter().any(|id| o["id"] == *id)))?;
        let id = found["id"].as_str()?.to_string();
        // Las opciones pueden venir agrupadas
        let options: Vec<AgentOption> = found["options"]
            .as_array()?
            .iter()
            .flat_map(|o| match o["options"].as_array() {
                Some(group) => group.iter().collect::<Vec<_>>(),
                None => vec![o],
            })
            .filter_map(|o| {
                Some(AgentOption {
                    id: o["value"].as_str()?.to_string(),
                    name: o["name"].as_str().unwrap_or_default().to_string(),
                    description: o["description"].as_str().map(str::to_string),
                })
            })
            .collect();
        let current = found["currentValue"].as_str().map(str::to_string);
        Some((id, options, current))
    };
    let mut config = AgentConfig::default();
    match select("model", &["model"]) {
        Some((id, options, current)) => {
            config.model_option = Some(id);
            config.models = options;
            config.current_model = current;
        }
        None => {
            (config.models, config.current_model) = options_of(
                v.get("models"),
                "availableModels",
                "currentModelId",
                "modelId",
            );
        }
    }
    if let Some((id, options, current)) = select("thought_level", &["thought_level", "effort"]) {
        config.effort_option = Some(id);
        config.efforts = options;
        config.current_effort = current;
    }
    config
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

    #[tokio::test]
    async fn conversa_por_acp_con_elicitacion_form_y_url() {
        async fn fake_elicitation_agent(stream: tokio::io::DuplexStream) {
            let (r, mut w) = tokio::io::split(stream);
            let mut lines = BufReader::new(r).lines();
            let mut prompt_id = None;
            while let Ok(Some(line)) = lines.next_line().await {
                let msg: Value = serde_json::from_str(&line).unwrap();
                let reply = match msg.get("method").and_then(Value::as_str) {
                    Some("initialize") => {
                        let cap = &msg["params"]["clientCapabilities"]["elicitation"];
                        assert!(cap["form"].is_object(), "debe anunciar elicitation.form");
                        assert!(cap["url"].is_object(), "debe anunciar elicitation.url");
                        vec![json!({"jsonrpc":"2.0","id":msg["id"],"result":{"protocolVersion":1}})]
                    }
                    Some("session/new") => {
                        vec![json!({"jsonrpc":"2.0","id":msg["id"],"result":{"sessionId":"ses-e"}})]
                    }
                    Some("session/prompt") => {
                        prompt_id = Some(msg["id"].clone());
                        vec![json!({
                            "jsonrpc": "2.0",
                            "id": "el-form",
                            "method": "elicitation/create",
                            "params": {
                                "sessionId": "ses-e",
                                "mode": "form",
                                "message": "¿Qué entorno?",
                                "requestedSchema": {
                                    "type": "object",
                                    "properties": {
                                        "env": { "type": "string", "enum": ["staging", "prod"] }
                                    },
                                    "required": ["env"]
                                }
                            }
                        })]
                    }
                    None if msg["id"] == "el-form" => {
                        assert_eq!(msg["result"]["action"], "accept");
                        assert_eq!(msg["result"]["content"]["env"], "staging");
                        vec![json!({
                            "jsonrpc": "2.0",
                            "id": "el-url",
                            "method": "elicitation/create",
                            "params": {
                                "sessionId": "ses-e",
                                "mode": "url",
                                "message": "Abre el enlace para continuar",
                                "url": "https://auth.example.com/device",
                                "elicitationId": "u-1"
                            }
                        })]
                    }
                    None if msg["id"] == "el-url" => {
                        assert_eq!(msg["result"]["action"], "decline");
                        assert!(msg["result"].get("content").is_none());
                        vec![json!({
                            "jsonrpc": "2.0",
                            "id": prompt_id.clone().unwrap(),
                            "result": {"stopReason": "end_turn"}
                        })]
                    }
                    _ => vec![],
                };
                for m in reply {
                    w.write_all(format!("{m}\n").as_bytes()).await.unwrap();
                }
            }
        }

        let events: Arc<StdMutex<Vec<(String, Value)>>> = Arc::default();
        let recorded = events.clone();
        let runner = AgentRunnerImpl::new(Arc::new(move |name, payload| {
            recorded.lock().unwrap().push((name.to_string(), payload));
        }));
        let (client, agent) = duplex(64 * 1024);
        tokio::spawn(fake_elicitation_agent(agent));
        let (r, w) = tokio::io::split(client);
        let session = runner.open_with(r, w, "/boveda").await.unwrap();
        assert_eq!(session.id, "ses-e");

        let prompt = {
            let runner = runner.clone();
            tokio::spawn(async move {
                runner
                    .prompt(
                        "ses-e",
                        vec![PromptBlock::Text {
                            text: "despliega".into(),
                        }],
                    )
                    .await
            })
        };

        let form_req = loop {
            let found = events.lock().unwrap().iter().find_map(|(n, p)| {
                (n == "acp://elicitation-request" && p["mode"] == "form")
                    .then(|| p["requestId"].as_str().unwrap().to_string())
            });
            if let Some(id) = found {
                break id;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        };
        runner
            .respond_elicitation(
                "ses-e",
                &form_req,
                ElicitationDecision::Accept {
                    content: Some(json!({"env": "staging"})),
                },
            )
            .await
            .unwrap();

        let url_req = loop {
            let found = events.lock().unwrap().iter().find_map(|(n, p)| {
                (n == "acp://elicitation-request" && p["mode"] == "url")
                    .then(|| p["requestId"].as_str().unwrap().to_string())
            });
            if let Some(id) = found {
                break id;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        };
        runner
            .respond_elicitation("ses-e", &url_req, ElicitationDecision::Decline)
            .await
            .unwrap();

        assert_eq!(prompt.await.unwrap().unwrap(), "end_turn");
    }

    /// Agente falso que abre una sesión distinta por cada `session/new`.
    async fn multi_session_agent(io: tokio::io::DuplexStream) {
        let (r, mut w) = tokio::io::split(io);
        let mut lines = BufReader::new(r).lines();
        let mut n = 0;
        while let Ok(Some(line)) = lines.next_line().await {
            let msg: Value = serde_json::from_str(&line).unwrap();
            let reply = match msg.get("method").and_then(Value::as_str) {
                Some("initialize") => {
                    json!({"jsonrpc":"2.0","id":msg["id"],"result":{"protocolVersion":1,
                        "agentCapabilities":{"sessionCapabilities":{"fork":{}}}}})
                }
                Some("session/fork") if msg["params"]["sessionId"] == "ses-1" => {
                    json!({"jsonrpc":"2.0","id":msg["id"],"result":{"sessionId":"ses-1-fork"}})
                }
                Some("session/fork") => {
                    json!({"jsonrpc":"2.0","id":msg["id"],"error":{"code":-1,"message":"no existe"}})
                }
                Some("session/new") => {
                    n += 1;
                    json!({"jsonrpc":"2.0","id":msg["id"],"result":{"sessionId": format!("ses-{n}")}})
                }
                _ => continue,
            };
            w.write_all(format!("{reply}\n").as_bytes()).await.unwrap();
        }
    }

    #[tokio::test]
    async fn las_conversaciones_comparten_proceso_hasta_el_limite() {
        let runner = AgentRunnerImpl::new(Arc::new(|_, _| {}))
            .with_idle_shutdown(std::time::Duration::from_millis(50));
        // Un comando que no existe: si el runner intentara lanzar otro proceso, fallaría
        let launch = AgentLaunch {
            command: "/no/existe/agente".into(),
            ..Default::default()
        };
        let (client, agent) = duplex(64 * 1024);
        tokio::spawn(multi_session_agent(agent));
        let (r, w) = tokio::io::split(client);
        let key = format!("{}#0", process_key("falso", "/b", &launch));
        runner.register_process(&key, r, w).await;

        let mut ids = vec![];
        for _ in 0..MAX_SESSIONS_PER_PROCESS {
            let s = runner
                .start("falso", launch.clone(), "/b", vec![], None, None, false)
                .await
                .unwrap();
            ids.push(s.id);
        }
        assert_eq!(ids, ["ses-1", "ses-2", "ses-3", "ses-4"]);
        assert_eq!(runner.process_count().await, 1);
        // La quinta ya no cabe: intenta lanzar otro proceso
        let err = runner
            .start("falso", launch.clone(), "/b", vec![], None, None, false)
            .await
            .unwrap_err();
        assert!(err.contains("No se pudo lanzar"), "{err}");

        // Con límite 1 por proceso, la siguiente conversación ya no cabe en el proceso existente
        runner.set_process_limit("falso", 1);
        let err = runner
            .start("falso", launch.clone(), "/b", vec![], None, None, false)
            .await
            .unwrap_err();
        assert!(err.contains("No se pudo lanzar"), "{err}");
        runner.set_process_limit("falso", MAX_SESSIONS_PER_PROCESS);

        // Cerrar una deja sitio en el mismo proceso
        runner.stop("ses-2").await.unwrap();
        let s = runner
            .start("falso", launch.clone(), "/b", vec![], None, None, false)
            .await
            .unwrap();
        assert_eq!(s.id, "ses-5");

        // Sin conversaciones, el proceso se apaga tras la espera
        for id in ["ses-1", "ses-3", "ses-4", "ses-5"] {
            runner.stop(id).await.unwrap();
        }
        assert_eq!(runner.process_count().await, 1);
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        assert_eq!(runner.process_count().await, 0);
    }

    #[tokio::test]
    async fn bifurca_en_el_agente_y_si_no_puede_abre_una_sesion_nueva() {
        let runner = AgentRunnerImpl::new(Arc::new(|_, _| {}));
        let launch = AgentLaunch {
            command: "/no/existe/agente".into(),
            ..Default::default()
        };
        let (client, agent) = duplex(64 * 1024);
        tokio::spawn(multi_session_agent(agent));
        let (r, w) = tokio::io::split(client);
        runner
            .register_process(&format!("{}#0", process_key("falso", "/b", &launch)), r, w)
            .await;
        let start = |resume: Option<&str>, fork| {
            let resume = resume.map(str::to_string);
            runner.start("falso", launch.clone(), "/b", vec![], None, resume, fork)
        };
        let first = start(None, false).await.unwrap();
        assert!(first.capabilities.fork_session && !first.resumed);

        let forked = start(Some("ses-1"), true).await.unwrap();
        assert_eq!((forked.id.as_str(), forked.resumed), ("ses-1-fork", true));

        // Sin contexto en el agente: la interfaz lo reenvía (`resumed` es falso)
        let fallback = start(Some("perdida"), true).await.unwrap();
        assert_eq!((fallback.id.as_str(), fallback.resumed), ("ses-2", false));
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
    fn lee_modelo_y_esfuerzo_de_config_options() {
        let r = json!({
            "models": {"currentModelId": "m-high", "availableModels": [{"modelId": "m-high", "name": "M (High)"}]},
            "configOptions": [
                {"id": "model", "category": "model", "type": "select", "currentValue": "m",
                 "options": [{"group": "g", "name": "G", "options": [{"value": "m", "name": "M"}]}, {"value": "n", "name": "N"}]},
                {"id": "effort", "category": "thought_level", "type": "select", "currentValue": "high",
                 "options": [{"value": "low", "name": "Low"}, {"value": "high", "name": "High"}]}]});
        let c = config_of(&r);
        assert_eq!(c.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["m", "n"]);
        assert_eq!(c.current_model.as_deref(), Some("m"));
        assert_eq!(c.model_option.as_deref(), Some("model"));
        assert_eq!(c.efforts.len(), 2);
        assert_eq!(c.current_effort.as_deref(), Some("high"));
        assert_eq!(c.effort_option.as_deref(), Some("effort"));

        // Sin configOptions: el modelo del bloque `models` y sin esfuerzo
        let old = config_of(&json!({"models": r["models"]}));
        assert_eq!(old.current_model.as_deref(), Some("m-high"));
        assert_eq!((old.model_option, old.efforts.len()), (None, 0));
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

    /// Modelo y esfuerzo con un agente real, sin enviarle mensajes:
    /// `SYNAPSE_REAL_AGENT=claude-acp cargo test -- --ignored --nocapture modelo_real`.
    #[tokio::test]
    #[ignore]
    async fn modelo_real() {
        use crate::application::integration::service::AgentInstaller;
        use crate::infrastructure::integration::service::acp::{
            AgentInstallerImpl, registry::RegistryClient,
        };
        let agent = std::env::var("SYNAPSE_REAL_AGENT").unwrap_or_else(|_| "claude-acp".into());
        let launch = AgentInstallerImpl::new(RegistryClient::new())
            .ensure_installed(&agent)
            .await
            .unwrap();
        let vault = tempfile::tempdir().unwrap();
        let cwd = vault.path().to_string_lossy().into_owned();
        let runner = AgentRunnerImpl::new(Arc::new(|_, _| {}));
        let s = runner
            .start(&agent, launch, &cwd, vec![], None, None, false)
            .await
            .unwrap();
        let c = s.config;
        println!("modelo {:?} de {}", c.current_model, c.models.len());
        println!("esfuerzo {:?} de {}", c.current_effort, c.efforts.len());
        let option = c.model_option.expect("sin opción de modelo");
        for m in &c.models {
            match runner.set_option(&s.id, &option, &m.id).await {
                Ok(n) => {
                    println!("{} -> {:?}, esfuerzos {}", m.id, n.current_model, n.efforts.len());
                    if let Some((o, e)) = n.effort_option.zip(n.efforts.first()) {
                        let r = runner.set_option(&s.id, &o, &e.id).await.unwrap();
                        println!("   {} -> {:?}", e.id, r.current_effort);
                    }
                }
                Err(e) => println!("{} -> ERROR {e}", m.id),
            }
        }
        runner.stop(&s.id).await.unwrap();
    }

    /// Prueba real con un agente instalado: `SYNAPSE_REAL_AGENT=claude-acp|antigravity|github-copilot`,
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
            .start(&agent, launch, &cwd, tools, Some(crate::infrastructure::entrypoint::controller::tauri::agent_controller::session_instructions(true)), None, false)
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
            session.config.models.iter().map(|m| &m.id).collect::<Vec<_>>(),
            session.config.current_model
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
        // Copia: el bloqueo no debe seguir vivo durante el `await` de abajo
        let events = events.lock().unwrap().clone();
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
