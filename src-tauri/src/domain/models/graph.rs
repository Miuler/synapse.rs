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
