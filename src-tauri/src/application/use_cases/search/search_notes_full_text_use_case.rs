use crate::application::dto::search::search_notes_full_text_query::{
    FullTextNoteHit, FullTextNoteSearch, SearchNotesFullTextQuery,
};
use crate::domain::graph::tag::Tag;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::search::full_text::FullTextQuery;
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
        let q = FullTextQuery {
            text: query.text.clone(),
            limit: query.limit * 4,
            prefix_last_term: true,
        };
        let (hits, total) = self
            .full_text_index
            .search(&q)
            .map_err(|err| format!("Error en full-text search: {}", err))?;

        let mut filtered = Vec::new();
        for h in hits {
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
            // Leer la nota para estimar líneas, tokens y verificar el filtro de etiquetas
            let (total_lines, estimated_tokens, matches_tags) =
                match NoteRelativePath::new(&h.note_path)
                    .ok()
                    .and_then(|rel| self.note_repository.read_note(&query.vault_path, &rel).ok())
                {
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
            filtered.push(FullTextNoteHit {
                hit: h,
                total_lines,
                estimated_tokens,
            });
            if filtered.len() >= query.limit {
                break;
            }
        }
        Ok(FullTextNoteSearch {
            total,
            hits: filtered,
        })
    }
}
