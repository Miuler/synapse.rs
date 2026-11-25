use super::docx_document::DocxDocument;

/// Lee un `.docx` desde sus bytes (de disco o de una revisión de Git), sin tocar el disco.
pub trait DocxParser: Send + Sync {
    fn parse(&self, bytes: &[u8]) -> Result<DocxDocument, String>;
    /// Bytes y tipo de la imagen con esa huella; `None` si no está o no se puede mostrar.
    fn image(&self, bytes: &[u8], sha256: &str) -> Result<Option<(String, Vec<u8>)>, String>;
}
