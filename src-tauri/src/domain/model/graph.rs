use serde::{Deserialize, Serialize};

/// Nodo en el grafo de conocimiento
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphNode {
    /// Ruta relativa normalizada (identificador único del nodo)
    pub id: String,
    /// Título legible para mostrar en el nodo
    pub label: String,
    /// Grupo o categoría para coloreado (ej. carpeta o tipo de archivo)
    pub group: String,
    /// Cantidad de enlaces entrantes (backlinks) para ponderar tamaño del nodo
    pub val: usize,
    /// Indica si es un nodo fantasma (nota referenciada que aún no existe en disco)
    #[serde(default)]
    pub is_ghost: bool,
}

/// Arista o conexión dirigida entre dos nodos
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GraphEdge {
    /// ID del nodo origen
    pub source: String,
    /// ID del nodo destino
    pub target: String,
}

/// Datos completos del grafo listos para `force-graph`
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VaultGraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphEdge>,
}

/// Elemento de backlink entrante hacia una nota
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BacklinkItem {
    pub source_path: String,
    pub source_title: String,
    pub snippet: String,
}

/// Elemento de outlink saliente desde una nota
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutlinkItem {
    pub target: String,
    pub alias: Option<String>,
    pub resolved_path: Option<String>,
    pub is_embed: bool,
}

/// Mención textual no vinculada que coincide con el título o nombre de la nota
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnlinkedMentionItem {
    pub source_path: String,
    pub source_title: String,
    pub snippet: String,
}

/// Informe consolidado de inspector de relaciones para una nota activa
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct NoteBacklinksReport {
    pub target_path: String,
    pub backlinks: Vec<BacklinkItem>,
    pub outlinks: Vec<OutlinkItem>,
    pub unlinked_mentions: Vec<UnlinkedMentionItem>,
}
