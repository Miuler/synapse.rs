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
    /// Puede bifurcar una conversación conservando su contexto (`session/fork`).
    #[serde(default)]
    pub fork_session: bool,
    /// Permite cambiar el nombre de sus sesiones (`session/set_title`).
    #[serde(default)]
    pub session_title: bool,
    /// Lista sus sesiones con su nombre (`session/list`).
    #[serde(default)]
    pub list_sessions: bool,
    /// Cierra una sesión liberando lo que tenga abierto para ella (`session/close`).
    #[serde(default)]
    pub close_session: bool,
}

/// Una opción elegible del agente (un modo o un modelo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentOption {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Modelo y esfuerzo de una sesión, y con qué opción se cambia cada uno.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub models: Vec<AgentOption>,
    pub current_model: Option<String>,
    /// Opción de configuración del modelo; sin ella se cambia con `session/set_model`.
    pub model_option: Option<String>,
    /// Niveles de esfuerzo (vacío si el agente o el modelo no lo ofrecen).
    pub efforts: Vec<AgentOption>,
    pub current_effort: Option<String>,
    pub effort_option: Option<String>,
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
    #[serde(flatten)]
    pub config: AgentConfig,
    /// Nombres de los servidores de herramientas conectados a la sesión.
    pub tool_servers: Vec<String>,
    /// La conversación se retomó en el agente (`session/load` o `session/fork`) y él conserva su historial.
    pub resumed: bool,
}
