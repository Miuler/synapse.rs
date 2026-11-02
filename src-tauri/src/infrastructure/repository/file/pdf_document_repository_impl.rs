use crate::domain::pdf::pdf_document::{PdfInspectionResult, PdfOutline, PdfPageSnippet};
use crate::domain::pdf::pdf_document_repository::PdfDocumentRepository;
use crate::infrastructure::search::pdf_extractor;
use std::fs;
use std::path::Path;

/// PDFs de la bóveda leídos del disco y procesados con `pdf_extractor`.
pub struct PdfDocumentRepositoryImpl;

impl PdfDocumentRepositoryImpl {
    fn read_bytes(vault_path: &Path, relative_path: &str) -> Result<Vec<u8>, String> {
        fs::read(vault_path.join(relative_path)).map_err(|e| format!("Error al leer PDF: {}", e))
    }
}

impl PdfDocumentRepository for PdfDocumentRepositoryImpl {
    fn outline(&self, vault_path: &Path, relative_path: &str) -> Result<PdfOutline, String> {
        let bytes = Self::read_bytes(vault_path, relative_path)?;
        let first_page = pdf_extractor::extract_pdf_page_range(&bytes, 1, 1, 100)?;
        Ok(PdfOutline {
            total_pages: first_page.total_pages,
            file_size_bytes: bytes.len(),
        })
    }

    fn read_pages(
        &self,
        vault_path: &Path,
        relative_path: &str,
        start_page: usize,
        end_page: usize,
        max_chars_per_page: usize,
    ) -> Result<PdfInspectionResult, String> {
        let bytes = Self::read_bytes(vault_path, relative_path)?;
        pdf_extractor::extract_pdf_page_range(&bytes, start_page, end_page, max_chars_per_page)
    }

    fn search(
        &self,
        vault_path: &Path,
        relative_path: &str,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<PdfPageSnippet>, String> {
        let bytes = Self::read_bytes(vault_path, relative_path)?;
        pdf_extractor::search_pdf_content(&bytes, query, max_results)
    }
}
