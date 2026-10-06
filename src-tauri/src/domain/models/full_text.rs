use serde::{Deserialize, Serialize};

/// Tipo de documento indexado en el motor de búsqueda full-text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentKind {
    Markdown,
    Mermaid,
    Code,
    Excalidraw,
}

impl DocumentKind {
    /// Identificador estable usado tanto en el índice como en el filtro `kind:xxx`.
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentKind::Markdown => "markdown",
            DocumentKind::Mermaid => "mermaid",
            DocumentKind::Code => "code",
            DocumentKind::Excalidraw => "excalidraw",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "markdown" => Some(DocumentKind::Markdown),
            "mermaid" => Some(DocumentKind::Mermaid),
            "code" => Some(DocumentKind::Code),
            "excalidraw" => Some(DocumentKind::Excalidraw),
            _ => None,
        }
    }
}

/// Documento listo para ser indexado (el texto ya fue extraído del formato original).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullTextDocument {
    /// Ruta relativa al vault normalizada con `/`. Es la clave única del documento.
    pub path: String,
    pub title: String,
    pub body: String,
    pub kind: DocumentKind,
    /// Fecha de modificación del archivo en nanosegundos (truncada a u64) para sync incremental.
    pub mtime_nanos: u64,
}

/// Consulta de búsqueda full-text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullTextQuery {
    /// Texto introducido por el usuario (sintaxis de QueryParser).
    pub text: String,
    /// Número máximo de resultados.
    pub limit: usize,
    /// Si es `true`, la última palabra se trata también como prefijo (búsqueda mientras se escribe).
    pub prefix_last_term: bool,
}

/// Fragmento de texto con indicador de resaltado.
///
/// Se envían segmentos ya cortados para que el frontend no tenga que convertir
/// offsets UTF-8 a UTF-16 ni recurrir a `{@html}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighlightSegment {
    pub text: String,
    pub highlighted: bool,
}

/// Resultado individual de una búsqueda full-text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTextHit {
    pub note_path: String,
    pub title: Vec<HighlightSegment>,
    pub snippet: Vec<HighlightSegment>,
    /// Términos (en su forma original en el documento) que coincidieron; útil para
    /// localizar la coincidencia en el editor al abrir la nota.
    pub matched_terms: Vec<String>,
    pub kind: DocumentKind,
    pub score: f32,
}

/// Estado del indexador en segundo plano.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FullTextIndexStatus {
    pub indexed_docs: u64,
    pub pending: usize,
    pub is_indexing: bool,
    pub in_memory_fallback: bool,
    pub last_error: Option<String>,
}

/// Respuesta completa de una búsqueda full-text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTextSearchResponse {
    pub hits: Vec<FullTextHit>,
    pub total_hits: usize,
    pub elapsed_ms: f64,
    pub status: FullTextIndexStatus,
}
