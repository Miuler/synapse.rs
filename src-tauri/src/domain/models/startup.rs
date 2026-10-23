use serde::{Deserialize, Serialize};

/// Representa el objetivo de apertura inicial de Synapse al arrancar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "path", rename_all = "snake_case")]
pub enum StartupTarget {
    /// Archivo puntual suelto (.md, .mermaid, .excalidraw) sin espacio de trabajo
    Standalone(String),
    /// Carpeta o directorio de bóveda/workspace completo
    Vault(String),
}
