use crate::application::dto::pdf::read_pdf_document_query::{PdfReading, ReadPdfDocumentQuery};
use crate::domain::repository::pdf_document_repository::PdfDocumentRepository;
use crate::domain::repository::vault_item_repository::VaultItemRepository;

/// Lee un PDF de la bóveda por páginas, busca dentro de él o devuelve su resumen.
pub struct ReadPdfDocumentUseCase<P: PdfDocumentRepository, V: VaultItemRepository> {
    pdfs: P,
    items: V,
}

impl<P: PdfDocumentRepository, V: VaultItemRepository> ReadPdfDocumentUseCase<P, V> {
    pub fn new(pdfs: P, items: V) -> Self {
        Self { pdfs, items }
    }

    pub fn execute(&self, query: ReadPdfDocumentQuery) -> Result<PdfReading, String> {
        let vault = &query.vault_path;
        let rel = query.relative_path.as_str();
        if !self.items.exists(vault, rel) {
            return Err(format!("El archivo PDF '{}' no existe", rel));
        }
        match query.mode.as_str() {
            "pages" => self
                .pdfs
                .read_pages(vault, rel, query.start_page, query.end_page, 20_000)
                .map(PdfReading::Pages),
            "search" => {
                let q = query
                    .search_query
                    .ok_or_else(|| "El argumento 'search_query' es obligatorio en mode='search'".to_string())?;
                let matches = self.pdfs.search(vault, rel, &q, 20)?;
                Ok(PdfReading::Search { query: q, matches })
            }
            _ => self.pdfs.outline(vault, rel).map(PdfReading::Outline),
        }
    }
}
