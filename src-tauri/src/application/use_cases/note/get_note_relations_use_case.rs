use crate::application::dto::note::get_note_relations_query::{Backlink, GetNoteRelationsQuery, NoteRelations, Outlink};
use crate::domain::model::wikilink::{LinkTarget, WikilinkToken};
use crate::domain::repository::note_repository::NoteRepository;
use std::collections::{HashMap, HashSet};

/// Relaciones de una nota: quién la enlaza, a quién enlaza y qué notas comparten etiquetas.
pub struct GetNoteRelationsUseCase<N: NoteRepository> {
    notes: N,
}

impl<N: NoteRepository> GetNoteRelationsUseCase<N> {
    pub fn new(notes: N) -> Self {
        Self { notes }
    }

    pub fn execute(&self, query: GetNoteRelationsQuery) -> Result<NoteRelations, String> {
        let extensions = vec!["md".to_string(), "markdown".to_string()];
        let all_notes = self
            .notes
            .list_notes(&query.vault_path, &extensions)
            .map_err(|e| format!("Error al listar notas: {}", e))?;

        let target_normalized = query.relative_path.trim_start_matches('/').replace('\\', "/");
        let target_stem = target_normalized
            .rsplit('/')
            .next()
            .and_then(|n| n.rsplit_once('.').map(|(s, _)| s))
            .unwrap_or(&target_normalized)
            .to_string();

        let mut target_tags: HashSet<String> = HashSet::new();
        let mut backlinks = Vec::new();
        let mut outlinks = Vec::new();
        let mut all_note_tags: HashMap<String, HashSet<String>> = HashMap::new();
        let all_note_paths: HashSet<String> = all_notes.iter().map(|n| n.relative_path.as_str().to_string()).collect();

        for note in &all_notes {
            let n_path = note.relative_path.as_str();
            let full_note = match self.notes.read_note(&query.vault_path, &note.relative_path) {
                Ok(n) => n,
                Err(_) => continue,
            };

            let note_tags_set: HashSet<String> = full_note
                .frontmatter
                .as_ref()
                .map(|fm| fm.tags.iter().map(|t| t.to_lowercase()).collect())
                .unwrap_or_default();

            if n_path == target_normalized {
                target_tags = note_tags_set.clone();
            }
            all_note_tags.insert(n_path.to_string(), note_tags_set);

            let wikilinks = WikilinkToken::extract_all(&full_note.content);
            let mut links_to_target = false;
            let mut matching_snippet = String::new();

            for wl in wikilinks {
                let (norm_target, _) = LinkTarget::normalize(&wl.target);
                if norm_target == target_normalized
                    || norm_target == format!("{}.md", target_stem)
                    || wl.target.eq_ignore_ascii_case(&target_stem)
                {
                    links_to_target = true;
                    // Extraer snippet de contexto de la línea
                    for line in full_note.content.lines() {
                        if line.contains(&format!("[[{}]]", wl.target)) || line.contains(&wl.target) {
                            matching_snippet = line.trim().to_string();
                            break;
                        }
                    }
                }

                if n_path == target_normalized {
                    let is_resolved = all_note_paths.contains(&norm_target) || all_note_paths.contains(&format!("{}.md", norm_target));
                    outlinks.push(Outlink {
                        target: wl.target,
                        alias: wl.alias,
                        is_embed: wl.is_embed,
                        is_resolved,
                    });
                }
            }

            if links_to_target && n_path != target_normalized {
                backlinks.push(Backlink {
                    source_path: n_path.to_string(),
                    title: note.title.clone(),
                    context_snippet: matching_snippet,
                });
            }
        }

        // Similitud Jaccard de etiquetas (Top 5 vecinos)
        let mut neighbors = Vec::new();
        if !target_tags.is_empty() {
            for (other_path, other_tags) in &all_note_tags {
                if other_path == &target_normalized || other_tags.is_empty() {
                    continue;
                }
                let intersection = target_tags.intersection(other_tags).count();
                let union = target_tags.union(other_tags).count();
                if union > 0 && intersection > 0 {
                    neighbors.push((other_path.clone(), (intersection as f64) / (union as f64)));
                }
            }
            neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            neighbors.truncate(5);
        }

        Ok(NoteRelations {
            target: target_normalized,
            backlinks,
            outlinks,
            neighbors,
        })
    }
}
