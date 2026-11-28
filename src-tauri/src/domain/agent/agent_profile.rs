use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// De dónde sale un agente: del catálogo verificado o definido por el usuario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentSource {
    Verified,
    Custom,
}

/// Cómo se lanza un agente: comando, argumentos y variables de entorno.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgentLaunch {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

/// Un agente disponible para el usuario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    pub source: AgentSource,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    /// Si el agente ya se puede lanzar (instalado, o personalizado).
    pub installed: bool,
    /// Solo en los personalizados; los verificados se resuelven al instalarlos.
    #[serde(default)]
    pub launch: Option<AgentLaunch>,
}

impl AgentProfile {
    /// Crea un agente personalizado validando sus datos mínimos.
    pub fn custom(id: &str, name: &str, launch: AgentLaunch) -> Result<Self, String> {
        let id = id.trim();
        let name = name.trim();
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(
                "agent ID may only contain letters, numbers, «-», and «_»".into(),
            );
        }
        if name.is_empty() {
            return Err("agent requires a name".into());
        }
        if launch.command.trim().is_empty() {
            return Err("agent requires a command".into());
        }
        Ok(Self {
            id: id.to_string(),
            name: name.to_string(),
            source: AgentSource::Custom,
            description: None,
            version: None,
            installed: true,
            launch: Some(launch),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_el_agente_personalizado() {
        let launch = AgentLaunch {
            command: "mi-agente".into(),
            ..Default::default()
        };
        assert!(AgentProfile::custom("mi-agente", "Mi agente", launch.clone()).is_ok());
        assert!(AgentProfile::custom("con espacio", "X", launch.clone()).is_err());
        assert!(AgentProfile::custom("a", " ", launch).is_err());
        assert!(AgentProfile::custom("a", "A", AgentLaunch::default()).is_err());
    }
}
