use crate::domain::pdf::pdf_document::{PdfInspectionResult, PdfOutline, PdfPageSnippet};
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

/// Permite pasar el repositorio por referencia a un caso de uso sin moverlo ni clonarlo.
impl<T: PdfDocumentRepository + ?Sized> PdfDocumentRepository for &T {
    fn outline(&self, vault_path: &Path, relative_path: &str) -> Result<PdfOutline, String> {
        (**self).outline(vault_path, relative_path)
    }

    fn read_pages(&self, vault_path: &Path, relative_path: &str, start_page: usize, end_page: usize, max_chars_per_page: usize,) -> Result<PdfInspectionResult, String> {
        (**self).read_pages(vault_path, relative_path, start_page, end_page, max_chars_per_page)
    }

    fn search(&self, vault_path: &Path, relative_path: &str, query: &str, max_results: usize,) -> Result<Vec<PdfPageSnippet>, String> {
        (**self).search(vault_path, relative_path, query, max_results)
    }
}
