use crate::application::dto::search::search_full_text_query::SearchFullTextQuery;
use crate::domain::search::full_text::{FullTextHit, FullTextQuery};
use crate::domain::search::full_text_index::FullTextIndex;
use std::sync::Arc;

/// Búsqueda de texto completo, sanitizando y acotando los parámetros de entrada.
pub struct SearchFullTextUseCase<I: FullTextIndex + ?Sized> {
    full_text_index: Arc<I>,
}

impl<I: FullTextIndex + ?Sized> SearchFullTextUseCase<I> {
    pub const MAX_QUERY_LEN: usize = 512;
    pub const DEFAULT_LIMIT: usize = 30;
    pub const MAX_LIMIT: usize = 200;

    pub fn new(full_text_index: Arc<I>) -> Self {
        Self { full_text_index }
    }

    pub fn execute(&self, query: SearchFullTextQuery) -> Result<(Vec<FullTextHit>, usize), String> {
        let trimmed = query.text.trim();
        if trimmed.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let bounded_query = if trimmed.len() > Self::MAX_QUERY_LEN {
            &trimmed[..Self::MAX_QUERY_LEN]
        } else {
            trimmed
        };

        let effective_limit = query
            .limit
            .unwrap_or(Self::DEFAULT_LIMIT)
            .clamp(1, Self::MAX_LIMIT);

        let full_text_query = FullTextQuery {
            text: bounded_query.to_string(),
            limit: effective_limit,
            prefix_last_term: true,
        };

        self.full_text_index.search(&full_text_query)
    }
}
