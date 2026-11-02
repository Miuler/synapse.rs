use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Relaciones semánticas estructuradas alineadas a la especificación OKF (Open Knowledge Format).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NoteRelations {
    /// Enlace jerárquico hacia arriba (contexto padre / tema principal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<String>,
    /// Enlaces semánticos relacionados en el mismo nivel
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
    /// Relaciones arbitrarias tipadas (e.g., depends_on, implements, references)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// Metadatos técnicos o de sincronización opcionales en el frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NoteSyncMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
}

/// Frontmatter YAML canónico en la cabecera de notas Markdown, compatible con OKF y MCP.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NoteFrontmatter {
    /// Identificador universal o portable del documento
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Título explícito de la nota
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Timestamp ISO 8601 de creación
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Timestamp ISO 8601 de última actualización
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// Tipo de nodo de conocimiento (note, concept, task, reference, daily, etc.)
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    /// Taxonomía y etiquetas
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Nombres alternativos o alias de búsqueda
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Grafo de conocimiento y relaciones semánticas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relations: Option<NoteRelations>,
    /// Metadatos de sincronización
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync: Option<NoteSyncMetadata>,
    /// Propiedades personalizadas adicionales definidas por el usuario
    #[serde(flatten)]
    pub extra_properties: HashMap<String, serde_yaml::Value>,
}

impl NoteFrontmatter {
    /// Determina si el frontmatter está completamente vacío.
    pub fn is_empty(&self) -> bool {
        self.id.is_none()
            && self.title.is_none()
            && self.created.is_none()
            && self.updated.is_none()
            && self.node_type.is_none()
            && self.tags.is_empty()
            && self.aliases.is_empty()
            && self.relations.is_none()
            && self.sync.is_none()
            && self.extra_properties.is_empty()
    }
}
