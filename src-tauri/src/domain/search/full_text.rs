use serde::{Deserialize, Serialize};

/// Tipo de documento indexado en el motor de búsqueda full-text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentKind {
    Markdown,
    Mermaid,
    Code,
    Excalidraw,
    Pdf,
    Word,
    /// Diagrama de draw.io: `.drawio` o una imagen (PNG/SVG) con el diagrama dentro.
    Drawio,
    /// Modelo ArchiMate (`.archimate`): nombres, documentación y propiedades de sus elementos.
    Archimate,
    /// Planilla (Excel, OpenDocument): hojas y filas.
    Excel,
    /// Diagrama de procesos BPMN (`.bpmn`): nombres de tareas, eventos y compuertas, y su documentación.
    Bpmn,
}

impl DocumentKind {
    /// Identificador estable usado tanto en el índice como en el filtro `kind:xxx`.
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentKind::Markdown => "markdown",
            DocumentKind::Mermaid => "mermaid",
            DocumentKind::Code => "code",
            DocumentKind::Excalidraw => "excalidraw",
            DocumentKind::Pdf => "pdf",
            DocumentKind::Word => "word",
            DocumentKind::Drawio => "drawio",
            DocumentKind::Archimate => "archimate",
            DocumentKind::Excel => "excel",
            DocumentKind::Bpmn => "bpmn",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "markdown" => Some(DocumentKind::Markdown),
            "mermaid" => Some(DocumentKind::Mermaid),
            "code" => Some(DocumentKind::Code),
            "excalidraw" => Some(DocumentKind::Excalidraw),
            "pdf" => Some(DocumentKind::Pdf),
            "word" => Some(DocumentKind::Word),
            "drawio" => Some(DocumentKind::Drawio),
            "archimate" => Some(DocumentKind::Archimate),
            "excel" => Some(DocumentKind::Excel),
            "bpmn" => Some(DocumentKind::Bpmn),
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
    /// Títulos y subtítulos de la nota (uno por línea). Vacío si el formato no tiene encabezados.
    pub headings: String,
    pub kind: DocumentKind,
    /// Fecha de modificación del archivo en nanosegundos (truncada a u64) para sync incremental.
    pub mtime_nanos: u64,
}

/// Consulta de búsqueda full-text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FullTextQuery {
    /// Texto introducido por el usuario (sintaxis de QueryParser, más `a|b` como `a OR b`).
    pub text: String,
    /// Número máximo de resultados.
    pub limit: usize,
    /// Si es `true`, la última palabra se trata también como prefijo (búsqueda mientras se escribe).
    pub prefix_last_term: bool,
    /// Devolver en cada resultado las líneas que coinciden (como `rg -n`).
    pub lines: Option<LineOptions>,
    /// Modo exacto: textos literales (sin distinguir mayúsculas) que el documento debe contener
    /// tal cual; `text` se ignora. Vacío = búsqueda normal.
    pub exact: Vec<String>,
}

/// Cuántas líneas devolver por documento y cuánto contexto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineOptions {
    /// Líneas antes y después de cada coincidencia.
    pub context: usize,
    /// Máximo de líneas por documento (el recuento total se informa aparte).
    pub max_per_doc: usize,
}

/// Dónde está una coincidencia, según el formato del documento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MatchLocation {
    /// Número de línea real del archivo (Markdown, código, diagramas en texto).
    Line { line: usize },
    /// Hoja de una planilla.
    Sheet { sheet: String },
    /// Elemento de un modelo (ArchiMate, BPMN): nombre y tipo.
    Element {
        element: String,
        #[serde(rename = "type")]
        element_type: String,
    },
}

/// Una línea que coincide, con su ubicación y contexto. Las líneas muy largas se recortan
/// alrededor de la coincidencia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchedLine {
    /// `None` si el formato no tiene una ubicación útil (PDF, Word, dibujos).
    pub location: Option<MatchLocation>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<String>,
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
    /// Encabezado encontrado al filtrar con `headings:`; vacío en otras búsquedas.
    /// `snippet` contiene entonces el texto que le sigue.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heading: Vec<HighlightSegment>,
    pub snippet: Vec<HighlightSegment>,
    /// Términos (en su forma original en el documento) que coincidieron; útil para
    /// localizar la coincidencia en el editor al abrir la nota.
    pub matched_terms: Vec<String>,
    pub kind: DocumentKind,
    pub score: f32,
    /// Líneas que coinciden, si se pidieron (`FullTextQuery::lines`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<MatchedLine>,
    /// Cuántas líneas coinciden en total, aunque `lines` se haya recortado.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub total_line_matches: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Estado del indexador en segundo plano.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FullTextIndexStatus {
    pub indexed_docs: u64,
    pub pending: usize,
    pub is_indexing: bool,
    pub in_memory_fallback: bool,
    pub last_error: Option<String>,
    pub current_file: Option<String>,
    pub total_to_index: usize,
    pub indexed_in_batch: usize,
    pub recent_files: Vec<String>,
}

/// Respuesta completa de una búsqueda full-text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTextSearchResponse {
    pub hits: Vec<FullTextHit>,
    pub total_hits: usize,
    pub elapsed_ms: f64,
    pub status: FullTextIndexStatus,
}
