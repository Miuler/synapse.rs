use crate::domain::model::pdf_document::{PdfInspectionResult, PdfOutline, PdfPageSnippet};
use std::path::Path;

/// Documentos PDF de la bóveda: lectura de su texto por páginas y búsqueda dentro de ellos.
pub trait PdfDocumentRepository: Send + Sync {
    fn outline(&self, vault_path: &Path, relative_path: &str) -> Result<PdfOutline, String>;
    fn read_pages(
        &self,
        vault_path: &Path,
        relative_path: &str,
        start_page: usize,
        end_page: usize,
        max_chars_per_page: usize,
    ) -> Result<PdfInspectionResult, String>;
    fn search(
        &self,
        vault_path: &Path,
        relative_path: &str,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<PdfPageSnippet>, String>;
}
