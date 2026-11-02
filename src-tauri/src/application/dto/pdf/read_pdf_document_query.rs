use crate::domain::pdf::pdf_document::{PdfInspectionResult, PdfOutline, PdfPageSnippet};
use std::path::PathBuf;

pub struct ReadPdfDocumentQuery {
    pub vault_path: PathBuf,
    pub relative_path: String,
    /// `pages`, `search` u `outline_only` (cualquier otro valor).
    pub mode: String,
    pub start_page: usize,
    /// Ya acotada a 5 páginas desde `start_page`.
    pub end_page: usize,
    pub search_query: Option<String>,
}

pub enum PdfReading {
    Pages(PdfInspectionResult),
    Search { query: String, matches: Vec<PdfPageSnippet> },
    Outline(PdfOutline),
}
