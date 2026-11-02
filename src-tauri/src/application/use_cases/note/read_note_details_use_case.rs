use crate::application::dto::note::read_note_details_query::{NoteDetails, ReadNoteDetailsQuery};
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;

/// Lee una nota con los datos que necesita un agente para navegarla sin cargarla entera.
pub struct ReadNoteDetailsUseCase<N: NoteRepository> {
    notes: N,
}

impl<N: NoteRepository> ReadNoteDetailsUseCase<N> {
    pub fn new(notes: N) -> Self {
        Self { notes }
    }

    pub fn execute(&self, query: ReadNoteDetailsQuery) -> Result<NoteDetails, String> {
        let relative_path = NoteRelativePath::new(&query.relative_path).map_err(|e| format!("Ruta inválida: {}", e))?;
        let note = self
            .notes
            .read_note(&query.vault_path, &relative_path)
            .map_err(|err| format!("Error al leer nota: {}", err))?;
        let lines: Vec<&str> = note.content.lines().collect();
        let (mermaid_count, d2_count) = NoteContent::diagram_block_counts(&lines);
        let details = NoteDetails {
            content_hash: NoteContent::hash(&note.content),
            total_lines: lines.len(),
            estimated_tokens: NoteContent::estimate_tokens(&note.content),
            toc: NoteContent::toc(&lines),
            mermaid_count,
            d2_count,
            note: note.clone(),
        };
        Ok(details)
    }
}
