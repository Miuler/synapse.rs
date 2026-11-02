/// Fragmento de texto de una página de un PDF.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PdfPageSnippet {
    pub page: u32,
    pub text: String,
}

/// Texto de un rango de páginas de un PDF.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PdfInspectionResult {
    pub total_pages: usize,
    pub pages: Vec<PdfPageSnippet>,
}

/// Resumen de un PDF: número de páginas y tamaño del archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfOutline {
    pub total_pages: usize,
    pub file_size_bytes: usize,
}
