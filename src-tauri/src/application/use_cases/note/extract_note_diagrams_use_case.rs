use crate::application::dto::note::extract_note_diagrams_query::{
    ExtractNoteDiagramsQuery, NoteDiagrams,
};
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Extrae los diagramas Mermaid y D2 de una nota o de un archivo de diagrama.
pub struct ExtractNoteDiagramsUseCase<N: NoteRepository, V: VaultItemRepository> {
    note_repository: N,
    vault_item_repository: V,
}

impl<N: NoteRepository, V: VaultItemRepository> ExtractNoteDiagramsUseCase<N, V> {
    pub fn new(note_repository: N, vault_item_repository: V) -> Self {
        Self {
            note_repository,
            vault_item_repository,
        }
    }

    pub fn execute(&self, query: ExtractNoteDiagramsQuery) -> Result<NoteDiagrams, String> {
        let relative_path = NoteRelativePath::new(&query.relative_path)
            .map_err(|e| format!("Ruta inválida: {}", e))?;

        if query.relative_path.ends_with(".d2") || query.relative_path.ends_with(".mmd") {
            let content = self
                .vault_item_repository
                .read_text(&query.vault_path, &query.relative_path)
                .map_err(|e| format!("Error al leer archivo de diagrama: {}", e))?;
            let kind = if query.relative_path.ends_with(".d2") {
                "d2"
            } else {
                "mermaid"
            };
            return Ok(NoteDiagrams::DedicatedFile {
                kind: kind.to_string(),
                content,
            });
        }

        let note = self
            .note_repository
            .read_note(&query.vault_path, &relative_path)
            .map_err(|e| format!("Error al leer nota: {}", e))?;
        let lines: Vec<&str> = note.content.lines().collect();
        Ok(NoteDiagrams::Blocks(NoteContent::diagrams(
            &lines,
            &query.diagram_type,
        )))
    }
}
