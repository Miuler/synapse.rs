use crate::application::dto::note::search_notes_query::{NoteSearchHit, SearchNotesQuery};
use crate::domain::graph::tag::Tag;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_repository::NoteRepository;

/// Busca notas por título, ruta, etiquetas o alias, con filtro opcional de etiquetas.
pub struct SearchNotesUseCase<N: NoteRepository> {
    note_repository: N,
}

impl<N: NoteRepository> SearchNotesUseCase<N> {
    pub fn new(note_repository: N) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, query: SearchNotesQuery) -> Result<Vec<NoteSearchHit>, String> {
        let extensions = vec!["md".to_string(), "markdown".to_string()];
        let notes = self
            .note_repository
            .list_notes(&query.vault_path, &extensions)
            .map_err(|err| format!("Error al buscar notas: {}", err))?;
        let text = query.query.as_str();

        Ok(notes
            .into_iter()
            .filter(|n| {
                // Extraer todas las etiquetas (YAML + inline hashtags)
                let all_tags = Tag::extract_all(&n.content);

                let matches_query = text.is_empty()
                    || n.title.to_lowercase().contains(text)
                    || n.relative_path.as_str().to_lowercase().contains(text)
                    || all_tags.iter().any(|t| Tag::matches(t, text))
                    || n.frontmatter.as_ref().is_some_and(|fm| {
                        fm.aliases.iter().any(|a| a.to_lowercase().contains(text))
                    });
                if !matches_query {
                    return false;
                }
                // Todas las tags especificadas deben coincidir jerárquicamente
                query.tags.iter().all(|tf| Tag::any_matches(&all_tags, tf))
            })
            .take(query.limit)
            .map(|n| {
                let description = n.frontmatter.as_ref().and_then(|fm| {
                    fm.extra_properties
                        .get("description")
                        .or_else(|| fm.extra_properties.get("summary"))
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                });
                NoteSearchHit {
                    snippet: NoteContent::clean_snippet(&n.content, 200),
                    total_lines: n.content.lines().count(),
                    estimated_tokens: NoteContent::estimate_tokens(&n.content),
                    tags: n.frontmatter.as_ref().map(|f| f.tags.clone()),
                    aliases: n.frontmatter.as_ref().map(|f| f.aliases.clone()),
                    description,
                    title: n.title,
                    relative_path: n.relative_path.as_str().to_string(),
                }
            })
            .collect())
    }
}
