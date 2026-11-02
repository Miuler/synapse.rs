use crate::application::dto::note::patch_note_command::{PatchNoteCommand, PatchNoteOutcome};
use crate::domain::note::note::Note;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;

/// Reemplaza un fragmento de una nota con tolerancia progresiva (saltos de línea, espacios,
/// indentación), respetando el control de concurrencia optimista.
pub struct PatchNoteUseCase<N: NoteRepository> {
    notes: N,
}

impl<N: NoteRepository> PatchNoteUseCase<N> {
    pub fn new(notes: N) -> Self {
        Self { notes }
    }

    pub fn execute(&self, command: PatchNoteCommand) -> Result<PatchNoteOutcome, String> {
        let relative_path = NoteRelativePath::new(&command.relative_path).map_err(|e| format!("Ruta inválida: {}", e))?;
        let note = self
            .notes
            .read_note(&command.vault_path, &relative_path)
            .map_err(|e| format!("Error al leer nota: {}", e))?;

        let current_hash = NoteContent::hash(&note.content);
        if let Some(exp) = command.expected_hash.as_deref() {
            if exp != current_hash {
                return Err(format!(
                    "Conflicto OCC en patch_note: el archivo cambió en disco. Hash actual: '{}', esperado: '{}'. Relee el documento.",
                    current_hash, exp
                ));
            }
        }

        let patched_content = NoteContent::apply_patch(&note.content, &command.old_text, &command.new_text, command.start_line)
            .map_err(|err_msg| format!("{} en '{}'", err_msg, command.relative_path))?;

        if command.dry_run {
            return Ok(PatchNoteOutcome {
                old_content: note.content,
                new_content: patched_content,
                new_hash: None,
            });
        }

        let new_hash = NoteContent::hash(&patched_content);
        let old_content = note.content.clone();
        let updated_note = Note::with_encoding(relative_path, "".to_string(), note.title, patched_content.clone(), note.encoding);
        self.notes
            .save_note(&command.vault_path, &updated_note)
            .map_err(|err| format!("Error al guardar parche: {}", err))?;
        Ok(PatchNoteOutcome {
            old_content,
            new_content: patched_content,
            new_hash: Some(new_hash),
        })
    }
}
