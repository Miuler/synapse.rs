/// Imagen ya codificada para devolverla por el MCP.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedImage {
    pub mime_type: String,
    pub data_base64: String,
}

/// Dibuja un diagrama de la bóveda (Excalidraw, Mermaid, D2, PlantUML…) como imagen. Lo hace la
/// ventana de Synapse, que tiene los motores de dibujo; sin ventana no hay quien lo dibuje.
pub trait DiagramRenderer: Send + Sync {
    /// Dibuja `relative_path` de la bóveda abierta, o `content` si se da (un diagrama de otra bóveda).
    fn render(&self, relative_path: &str, content: Option<&str>) -> Result<RenderedImage, String>;
}

/// Extensiones de los diagramas que Synapse sabe dibujar.
pub const DIAGRAM_EXTENSIONS: &[&str] = &[
    ".excalidraw.excalidraw",
    ".excalidraw.json",
    ".excalidraw.md",
    ".excalidraw",
    ".mermaid",
    ".mmd",
    ".d2",
    ".puml",
    ".plantuml",
];

/// Tipo de diagrama según la extensión, o `None` si no es un diagrama.
pub fn diagram_kind(relative_path: &str) -> Option<&'static str> {
    let lower = relative_path.to_lowercase();
    let ext = DIAGRAM_EXTENSIONS.iter().find(|e| lower.ends_with(*e))?;
    Some(match *ext {
        ".mermaid" | ".mmd" => "mermaid",
        ".d2" => "d2",
        ".puml" | ".plantuml" => "plantuml",
        _ => "excalidraw",
    })
}

#[cfg(test)]
mod tests {
    use super::diagram_kind;

    #[test]
    fn reconoce_los_diagramas_por_extension() {
        assert_eq!(diagram_kind("a/Flujo.d2"), Some("d2"));
        assert_eq!(diagram_kind("x.MMD"), Some("mermaid"));
        assert_eq!(diagram_kind("dibujo.excalidraw.md"), Some("excalidraw"));
        assert_eq!(diagram_kind("seq.puml"), Some("plantuml"));
        assert_eq!(diagram_kind("nota.md"), None);
    }
}
