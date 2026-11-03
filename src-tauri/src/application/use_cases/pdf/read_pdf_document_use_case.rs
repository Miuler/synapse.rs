use crate::application::dto::pdf::read_pdf_document_query::{PdfReading, ReadPdfDocumentQuery};
use crate::domain::pdf::pdf_document_repository::PdfDocumentRepository;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Lee un PDF de la bóveda por páginas, busca dentro de él o devuelve su resumen.
pub struct ReadPdfDocumentUseCase<P: PdfDocumentRepository, V: VaultItemRepository> {
    pdf_document_repository: P,
    vault_item_repository: V,
}

impl<P: PdfDocumentRepository, V: VaultItemRepository> ReadPdfDocumentUseCase<P, V> {
    pub fn new(pdf_document_repository: P, vault_item_repository: V) -> Self {
        Self {
            pdf_document_repository,
            vault_item_repository,
        }
    }

    pub fn execute(&self, query: ReadPdfDocumentQuery) -> Result<PdfReading, String> {
        let vault = &query.vault_path;
        let rel = query.relative_path.as_str();
        if !self.vault_item_repository.exists(vault, rel) {
            return Err(format!("El archivo PDF '{}' no existe", rel));
        }
        match query.mode.as_str() {
            "pages" => self
                .pdf_document_repository
                .read_pages(vault, rel, query.start_page, query.end_page, 20_000)
                .map(PdfReading::Pages),
            "search" => {
                let q = query.search_query.ok_or_else(|| {
                    "El argumento 'search_query' es obligatorio en mode='search'".to_string()
                })?;
                let matches = self.pdf_document_repository.search(vault, rel, &q, 20)?;
                Ok(PdfReading::Search { query: q, matches })
            }
            _ => self
                .pdf_document_repository
                .outline(vault, rel)
                .map(PdfReading::Outline),
        }
    }
}
