use crate::application::dto::note::search_in_note_query::{LineMatch, SearchInNoteQuery};
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;

/// Busca un texto dentro de una nota (sin distinguir mayúsculas) y devuelve cada coincidencia
/// con sus líneas de contexto.
pub struct SearchInNoteUseCase<N: NoteRepository> {
    note_repository: N,
}

impl<N: NoteRepository> SearchInNoteUseCase<N> {
    pub fn new(note_repository: N) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, query: SearchInNoteQuery) -> Result<Vec<LineMatch>, String> {
        let context_lines = query.context_lines.min(10);
        let relative_path = NoteRelativePath::new(&query.relative_path)
            .map_err(|e| format!("Ruta inválida: {}", e))?;
        let note = self
            .note_repository
            .read_note(&query.vault_path, &relative_path)
            .map_err(|e| format!("Error al leer nota: {}", e))?;

        let lines: Vec<&str> = note.content.lines().collect();
        let query_lower = query.query.to_lowercase();
        let mut matches = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                let start_idx = i.saturating_sub(context_lines);
                let end_idx = (i + context_lines + 1).min(lines.len());
                matches.push(LineMatch {
                    line_number: i + 1,
                    matched_line: line.to_string(),
                    context: (start_idx..end_idx)
                        .map(|idx| format!("{}: {}", idx + 1, lines[idx]))
                        .collect(),
                });
            }
        }
        Ok(matches)
    }
}
