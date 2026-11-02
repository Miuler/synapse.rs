use crate::application::dto::note::write_note_command::{WriteNoteCommand, WriteNoteOutcome};
use crate::domain::model::frontmatter::NoteFrontmatter;
use crate::domain::model::note::Note;
use crate::domain::model::note_content::NoteContent;
use crate::domain::model::note_path::NoteRelativePath;
use crate::domain::repository::note_repository::NoteRepository;

/// Escribe una nota completa. Rechaza la escritura si la nota cambió desde que se leyó
/// (`expected_hash`) o si recortaría drásticamente una nota grande sin permiso explícito.
pub struct WriteNoteUseCase<N: NoteRepository> {
    notes: N,
}

impl<N: NoteRepository> WriteNoteUseCase<N> {
    pub fn new(notes: N) -> Self {
        Self { notes }
    }

    pub fn execute(&self, command: WriteNoteCommand) -> Result<WriteNoteOutcome, String> {
        let relative_path = NoteRelativePath::new(&command.relative_path).map_err(|e| format!("Ruta inválida: {}", e))?;
        let content = command.content.as_str();

        // Leer nota existente si existe
        let existing_note = self.notes.read_note(&command.vault_path, &relative_path).ok();

        if let Some(ref note) = existing_note {
            let current_hash = NoteContent::hash(&note.content);
            if let Some(exp) = command.expected_hash.as_deref() {
                if exp != current_hash {
                    return Err(format!(
                        "Conflicto de concurrencia OCC: el archivo cambió en disco. Hash actual: '{}', esperado: '{}'. Relee el documento antes de escribir.",
                        current_hash, exp
                    ));
                }
            }

            // Freno anti-truncamiento (>150 líneas y reduce más del 40%)
            let old_line_count = note.content.lines().count();
            let new_line_count = content.lines().count();
            if old_line_count > 150 && !command.allow_large_overwrite && new_line_count < (old_line_count * 60) / 100 {
                return Err(format!(
                    "Freno anti-truncamiento activado: la nota original tiene {} líneas y la nueva tiene {}. Usa patch_note para ediciones parciales o establece allow_large_overwrite: true.",
                    old_line_count, new_line_count
                ));
            }
        }

        let title = command.title.clone().unwrap_or_else(|| {
            existing_note
                .as_ref()
                .map(|n| n.title.clone())
                .unwrap_or_else(|| "Sin título".to_string())
        });

        let final_body = if content.trim_start().starts_with("---") {
            content.to_string()
        } else {
            let mut fm = existing_note
                .as_ref()
                .and_then(|n| n.frontmatter.clone())
                .unwrap_or_default();
            fm.title = Some(title.clone());
            if let Some(tags) = command.tags {
                fm.tags = tags;
            }
            if let Some(aliases) = command.aliases {
                fm.aliases = aliases;
            }
            NoteFrontmatter::serialize_with_content(Some(&fm), content)
        };

        if command.dry_run {
            let old_content = existing_note.map(|n| n.content).unwrap_or_default();
            return Ok(WriteNoteOutcome::Preview {
                old_content,
                new_content: final_body,
            });
        }

        let note = Note::with_encoding(relative_path, "".to_string(), title, final_body.clone(), "UTF-8".to_string());
        self.notes
            .save_note(&command.vault_path, &note)
            .map_err(|err| format!("Error al guardar la nota: {}", err))?;
        Ok(WriteNoteOutcome::Saved {
            new_hash: NoteContent::hash(&final_body),
        })
    }
}
