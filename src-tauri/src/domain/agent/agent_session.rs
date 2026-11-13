use serde::{Deserialize, Serialize};

/// Lo que un agente sabe hacer, según lo que anuncia al conectarse.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Acepta imágenes en el prompt.
    pub image: bool,
    /// Acepta contenido incrustado (texto de una selección o de un archivo).
    pub embedded_context: bool,
    /// Puede conectarse a servidores de herramientas por HTTP.
    pub tools_over_http: bool,
    /// Puede retomar conversaciones anteriores.
    pub load_session: bool,
}

/// Una opción elegible del agente (un modo o un modelo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentOption {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Una conversación abierta con un agente sobre una carpeta de trabajo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub agent_id: String,
    pub cwd: String,
    pub capabilities: AgentCapabilities,
    pub modes: Vec<AgentOption>,
    pub current_mode: Option<String>,
    pub models: Vec<AgentOption>,
    pub current_model: Option<String>,
    /// Nombres de los servidores de herramientas conectados a la sesión.
    pub tool_servers: Vec<String>,
    /// La conversación se retomó en el agente (`session/load`) y él conserva su historial.
    pub resumed: bool,
}
