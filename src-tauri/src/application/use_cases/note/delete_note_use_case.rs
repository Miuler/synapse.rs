use crate::application::dto::note::delete_note_command::DeleteNoteCommand;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;

/// Elimina una nota; si se indica `expected_hash`, solo si no cambió desde que se leyó.
pub struct DeleteNoteUseCase<N: NoteRepository> {
    notes: N,
}

impl<N: NoteRepository> DeleteNoteUseCase<N> {
    pub fn new(notes: N) -> Self {
        Self { notes }
    }

    pub fn execute(&self, command: DeleteNoteCommand) -> Result<(), String> {
        let note_path = NoteRelativePath::new(&command.relative_path)?;

        if let Some(expected_hash) = command.expected_hash.as_deref() {
            let note = self
                .notes
                .read_note(&command.vault_path, &note_path)
                .map_err(|e| format!("No se pudo leer la nota para validar el hash: {}", e))?;
            let current_hash = NoteContent::hash(&note.content);
            if current_hash != expected_hash {
                return Err(format!(
                    "Conflicto de concurrencia optimista (OCC): el hash esperado '{}' no coincide con el hash actual de la nota '{}'",
                    expected_hash, current_hash
                ));
            }
        }

        self.notes
            .delete_note(&command.vault_path, &note_path)
            .map_err(|err| format!("Error al eliminar nota: {}", err))
    }
}
