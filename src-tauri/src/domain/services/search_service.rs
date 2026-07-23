use crate::domain::models::note::Note;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub text: String,
    pub score: u32,
    pub match_indices: Vec<u32>,
    pub note_path: Option<String>,
}

pub trait SearchService: Send + Sync {
    /// Búsqueda rápida para comandos o títulos
    fn search_items(&self, query: &str, items: &[String]) -> Vec<SearchResult>;

    /// Búsqueda profunda en contenido y títulos de notas
    fn search_notes(&self, query: &str, notes: &[Note]) -> Vec<SearchResult>;
}
