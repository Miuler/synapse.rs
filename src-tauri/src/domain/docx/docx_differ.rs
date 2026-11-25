use super::docx_diff::DocxDiffEntry;
use super::docx_document::DocxDocument;

/// Compara dos documentos bloque a bloque.
pub trait DocxDiffer: Send + Sync {
    /// Las diferencias y si se agotó el tiempo (resultado aproximado).
    fn diff(&self, old: &DocxDocument, new: &DocxDocument) -> (Vec<DocxDiffEntry>, bool);
}
