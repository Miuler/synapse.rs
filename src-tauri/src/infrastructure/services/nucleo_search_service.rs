use crate::domain::models::note::Note;
use crate::domain::services::search_service::{SearchResult, SearchService};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

pub struct NucleoSearchService;

impl NucleoSearchService {
    pub fn new() -> Self {
        Self
    }
}

impl SearchService for NucleoSearchService {
    fn search_items(&self, query: &str, items: &[String]) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return items
                .iter()
                .map(|item| SearchResult {
                    text: item.clone(),
                    score: 0,
                    match_indices: Vec::new(),
                    note_path: None,
                })
                .collect();
        }

        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut results = Vec::new();

        for item in items {
            let mut buf = Vec::new();
            let utf32_item = Utf32Str::new(item, &mut buf);
            let mut indices = Vec::new();

            if let Some(score) = pattern.indices(utf32_item, &mut matcher, &mut indices) {
                results.push(SearchResult {
                    text: item.clone(),
                    score: score as u32,
                    match_indices: indices.into_iter().map(|idx| idx as u32).collect(),
                    note_path: None,
                });
            }
        }

        // Ordenar de mayor a menor puntuación
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    fn search_notes(&self, query: &str, notes: &[Note]) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return notes
                .iter()
                .map(|n| SearchResult {
                    text: n.title.clone(),
                    score: 0,
                    match_indices: Vec::new(),
                    note_path: Some(n.relative_path.as_str().to_string()),
                })
                .collect();
        }

        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut results = Vec::new();

        for note in notes {
            // 1. Buscar en el título
            let mut buf_title = Vec::new();
            let utf32_title = Utf32Str::new(&note.title, &mut buf_title);
            let mut indices_title = Vec::new();

            let title_score = pattern.indices(utf32_title, &mut matcher, &mut indices_title);

            // 2. Buscar en el contenido
            let mut buf_content = Vec::new();
            let utf32_content = Utf32Str::new(&note.content, &mut buf_content);
            let mut indices_content = Vec::new();

            let content_score = pattern.indices(utf32_content, &mut matcher, &mut indices_content);

            if title_score.is_some() || content_score.is_some() {
                let score = title_score.unwrap_or(0).max(content_score.unwrap_or(0));
                let indices = if title_score.is_some() {
                    indices_title
                } else {
                    indices_content
                };

                results.push(SearchResult {
                    text: format!("{} - {}", note.title, note.relative_path.as_str()),
                    score: score as u32,
                    match_indices: indices.into_iter().map(|idx| idx as u32).collect(),
                    note_path: Some(note.relative_path.as_str().to_string()),
                });
            }
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }
}
