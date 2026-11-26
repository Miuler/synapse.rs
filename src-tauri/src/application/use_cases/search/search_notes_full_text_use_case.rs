use crate::application::dto::search::search_notes_full_text_query::{
    FullTextNoteHit, FullTextNoteSearch, SearchNotesFullTextQuery,
};
use crate::domain::graph::tag::Tag;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::search::full_text::{DocumentKind, FullTextQuery};
use crate::domain::search::full_text_index::FullTextIndex;

/// Búsqueda de texto completo con filtros de carpeta y etiquetas, enriquecida con el tamaño de
/// cada nota.
pub struct SearchNotesFullTextUseCase<I: FullTextIndex, N: NoteRepository> {
    full_text_index: I,
    note_repository: N,
}

impl<I: FullTextIndex, N: NoteRepository> SearchNotesFullTextUseCase<I, N> {
    pub fn new(full_text_index: I, note_repository: N) -> Self {
        Self {
            full_text_index,
            note_repository,
        }
    }

    pub fn execute(&self, query: SearchNotesFullTextQuery) -> Result<FullTextNoteSearch, String> {
        // Con filtros de carpeta o extensión se piden muchos más candidatos: se filtran después,
        // y si los mejores estaban en otra carpeta, la pedida quedaba vacía aunque tuviera resultados.
        let filtered_search = query.folder.is_some() || !query.extensions.is_empty();
        let q = FullTextQuery {
            text: query.text.clone(),
            limit: if filtered_search { (query.limit * 50).max(1_000) } else { query.limit * 4 },
            prefix_last_term: true,
            lines: query.lines,
            exact: query.exact.clone(),
        };
        let (hits, total) = self
            .full_text_index
            .search(&q)
            .map_err(|err| format!("Error en full-text search: {}", err))?;

        let mut filtered = Vec::new();
        let mut in_scope = 0;
        for h in hits {
            if !query.extensions.is_empty() {
                let lower = h.note_path.to_lowercase();
                if !query.extensions.iter().any(|e| lower.ends_with(&format!(".{e}"))) {
                    continue;
                }
            }
            if let Some(ref ff) = query.folder {
                let lower_path = h.note_path.to_lowercase();
                let is_prefix = lower_path.starts_with(ff.as_str());
                let is_segment = lower_path.contains(&format!("/{}/", ff))
                    || lower_path.starts_with(&format!("{}/", ff))
                    || lower_path.contains(&format!("/{}", ff));
                if !is_prefix && !is_segment {
                    continue;
                }
            }
            in_scope += 1;
            if filtered.len() >= query.limit {
                continue;
            }
            // Leer la nota para estimar líneas, tokens y verificar el filtro de etiquetas
            let note = NoteRelativePath::new(&h.note_path)
                .ok()
                .and_then(|rel| self.note_repository.read_note(&query.vault_path, &rel).ok());
            let (total_lines, estimated_tokens, matches_tags) = match &note {
                Some(note) => {
                    let ok_tags = query.tags.is_empty() || {
                        let all_tags = Tag::extract_all(&note.content);
                        query.tags.iter().all(|tf| Tag::any_matches(&all_tags, tf))
                    };
                    (
                        note.content.lines().count(),
                        NoteContent::estimate_tokens(&note.content),
                        ok_tags,
                    )
                }
                None => (0, 0, query.tags.is_empty()),
            };
            if !matches_tags {
                continue;
            }
            let mut h = h;
            if query.lines.is_some() {
                // En los formatos de texto, sobre el archivo: el índice guarda un texto procesado
                // (en Markdown, sin frontmatter y un párrafo por línea) y los números no coincidirían
                let plain_text = matches!(h.kind, DocumentKind::Markdown | DocumentKind::Mermaid | DocumentKind::Code);
                (h.lines, h.total_line_matches) = match (&note, plain_text) {
                    (Some(note), true) => self.full_text_index.match_lines(&q, &note.content),
                    _ => self.full_text_index.match_document_lines(&q, &h.note_path),
                };
            }
            filtered.push(FullTextNoteHit {
                hit: h,
                total_lines,
                estimated_tokens,
            });
        }
        Ok(FullTextNoteSearch {
            // Con filtros, el total es el de la carpeta/extensiones pedidas, no el de toda la bóveda
            total: if filtered_search { in_scope } else { total },
            hits: filtered,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::search::full_text::{DocumentKind, FullTextDocument, FullTextHit};
    use crate::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
    use std::collections::HashMap;

    /// Índice simulado: 30 coincidencias en `otros/` (mejor puntuadas) y 2 en `src/soluciones/`.
    struct FakeIndex;
    impl FullTextIndex for FakeIndex {
        fn upsert(&self, _: Vec<FullTextDocument>) -> Result<(), String> { Ok(()) }
        fn delete(&self, _: &[String]) -> Result<(), String> { Ok(()) }
        fn delete_all(&self) -> Result<(), String> { Ok(()) }
        fn commit(&self) -> Result<(), String> { Ok(()) }
        fn indexed_versions(&self) -> Result<HashMap<String, u64>, String> { Ok(HashMap::new()) }
        fn num_docs(&self) -> u64 { 32 }
        fn match_lines(&self, _: &FullTextQuery, _: &str) -> (Vec<crate::domain::search::full_text::MatchedLine>, usize) { (vec![], 0) }
        fn match_document_lines(&self, _: &FullTextQuery, _: &str) -> (Vec<crate::domain::search::full_text::MatchedLine>, usize) { (vec![], 0) }
        fn search(&self, q: &FullTextQuery) -> Result<(Vec<FullTextHit>, usize), String> {
            let hit = |path: String, kind| FullTextHit {
                note_path: path,
                title: vec![],
                heading: vec![],
                snippet: vec![],
                matched_terms: vec![],
                kind,
                score: 1.0,
                lines: vec![],
                total_line_matches: 0,
                truncated: false,
            };
            let mut hits: Vec<_> = (0..30).map(|i| hit(format!("otros/n{i}.md"), DocumentKind::Markdown)).collect();
            hits.push(hit("src/soluciones/a.archimate".into(), DocumentKind::Archimate));
            hits.push(hit("src/soluciones/b.archimate".into(), DocumentKind::Archimate));
            let total = hits.len();
            hits.truncate(q.limit);
            Ok((hits, total))
        }
    }

    fn query(folder: Option<&str>, extensions: &[&str]) -> SearchNotesFullTextQuery {
        SearchNotesFullTextQuery {
            vault_path: std::env::temp_dir(),
            text: "maintenance".into(),
            limit: 5,
            folder: folder.map(str::to_string),
            tags: vec![],
            extensions: extensions.iter().map(|e| e.to_string()).collect(),
            lines: None,
            exact: vec![],
        }
    }

    #[test]
    fn la_carpeta_pedida_no_queda_vacia_aunque_los_mejores_esten_en_otra() {
        let uc = SearchNotesFullTextUseCase::new(FakeIndex, NoteRepositoryImpl::new());
        let r = uc.execute(query(Some("src/soluciones"), &[])).unwrap();
        assert_eq!(r.hits.len(), 2);
        // El total es el de la carpeta, no el de toda la bóveda
        assert_eq!(r.total, 2);
    }

    #[test]
    fn filtra_por_extension() {
        let uc = SearchNotesFullTextUseCase::new(FakeIndex, NoteRepositoryImpl::new());
        let r = uc.execute(query(None, &["archimate"])).unwrap();
        assert_eq!(r.hits.iter().map(|h| h.hit.note_path.as_str()).collect::<Vec<_>>(), vec!["src/soluciones/a.archimate", "src/soluciones/b.archimate"]);
        let sin_filtro = uc.execute(query(None, &[])).unwrap();
        assert_eq!((sin_filtro.hits.len(), sin_filtro.total), (5, 32));
    }
}
