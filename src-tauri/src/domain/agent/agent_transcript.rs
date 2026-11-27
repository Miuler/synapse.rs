use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Conversación guardada con un agente, para revisarla o retomarla más tarde.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentTranscript {
    pub id: String,
    pub agent_id: String,
    #[serde(default)]
    pub agent_name: String,
    pub cwd: String,
    #[serde(default)]
    pub title: String,
    /// Sesión ACP del agente, para pedirle que la retome (`session/load`).
    #[serde(default)]
    pub acp_session_id: Option<String>,
    /// Milisegundos desde la época Unix.
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    /// Contexto usado la última vez, para verlo al retomarla (el agente no lo repite hasta responder).
    #[serde(default)]
    pub usage: Value,
    /// Mensajes tal como los muestra la interfaz.
    #[serde(default)]
    pub items: Value,
    /// Nombre que el agente le da a la sesión.
    #[serde(default)]
    pub native_title: Option<String>,
    /// Preguntas aparte (`/btw`) de la conversación, cada una con sus mensajes y su sesión.
    #[serde(default)]
    pub asides: Value,
}

/// Datos de una conversación guardada, sin sus mensajes, para listarla.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentTranscriptSummary {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub cwd: String,
    pub title: String,
    pub acp_session_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<&AgentTranscript> for AgentTranscriptSummary {
    fn from(t: &AgentTranscript) -> Self {
        Self {
            id: t.id.clone(),
            agent_id: t.agent_id.clone(),
            agent_name: t.agent_name.clone(),
            cwd: t.cwd.clone(),
            title: t.title.clone(),
            acp_session_id: t.acp_session_id.clone(),
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

impl AgentTranscript {
    /// Identificador válido como nombre de archivo (letras, dígitos, `-` y `_`).
    pub fn valid_id(id: &str) -> bool {
        !id.is_empty()
            && id.len() <= 128
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }
}
