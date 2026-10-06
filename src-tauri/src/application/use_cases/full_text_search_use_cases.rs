use crate::domain::models::full_text::{FullTextHit, FullTextQuery};
use crate::domain::services::full_text_index::FullTextIndex;
use std::sync::Arc;

/// Casos de uso para búsqueda de texto completo.
pub struct FullTextSearchUseCases<I: FullTextIndex + ?Sized> {
    index: Arc<I>,
}

impl<I: FullTextIndex + ?Sized> FullTextSearchUseCases<I> {
    pub const MAX_QUERY_LEN: usize = 512;
    pub const DEFAULT_LIMIT: usize = 30;
    pub const MAX_LIMIT: usize = 200;

    pub fn new(index: Arc<I>) -> Self {
        Self { index }
    }

    /// Realiza una búsqueda de texto completo sanitizando y acotando los parámetros de entrada.
    pub fn search(&self, text: &str, limit: Option<usize>) -> Result<(Vec<FullTextHit>, usize), String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let bounded_query = if trimmed.len() > Self::MAX_QUERY_LEN {
            &trimmed[..Self::MAX_QUERY_LEN]
        } else {
            trimmed
        };

        let effective_limit = limit.unwrap_or(Self::DEFAULT_LIMIT).clamp(1, Self::MAX_LIMIT);

        let query = FullTextQuery {
            text: bounded_query.to_string(),
            limit: effective_limit,
            prefix_last_term: true,
        };

        self.index.search(&query)
    }

    pub fn num_docs(&self) -> u64 {
        self.index.num_docs()
    }
}
