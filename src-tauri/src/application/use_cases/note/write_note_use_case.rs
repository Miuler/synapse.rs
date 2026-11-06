use crate::application::dto::note::write_note_command::{WriteNoteCommand, WriteNoteOutcome};
use crate::domain::note::frontmatter::NoteFrontmatter;
use crate::domain::note::note::Note;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;

/// Escribe una nota completa. Rechaza la escritura si la nota ya existe y no se autorizó
/// reemplazarla (`expected_hash` u `overwrite`), si cambió desde que se leyó o si recortaría drásticamente una nota grande sin permiso explícito.
pub struct WriteNoteUseCase<N: NoteRepository> {
    note_repository: N,
}

impl<N: NoteRepository> WriteNoteUseCase<N> {
    pub fn new(note_repository: N) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, command: WriteNoteCommand) -> Result<WriteNoteOutcome, String> {
        let relative_path = NoteRelativePath::new(&command.relative_path)
            .map_err(|e| format!("Ruta inválida: {}", e))?;
        let content = command.content.as_str();

        // Leer nota existente si existe
        let existing_note = match self
            .note_repository
            .read_note(&command.vault_path, &relative_path)
        {
            Ok(note) => Some(note),
            Err(e) if e.starts_with("El archivo no existe") => None,
            // Existe pero no se pudo leer: tratarla como nueva la destruiría.
            Err(e) => return Err(format!("No se pudo leer la nota existente: {}", e)),
        };

        if existing_note.is_none() && command.expected_hash.is_some() {
            return Err(format!(
                "Se indicó expected_hash, pero la nota '{}' no existe: pudo moverse o borrarse. Comprueba la ruta; para crearla, omite expected_hash.",
                command.relative_path
            ));
        }

        if let Some(ref note) = existing_note {
            let current_hash = NoteContent::hash(&note.content);
            if let Some(exp) = command.expected_hash.as_deref() {
                if exp != current_hash {
                    return Err(format!(
                        "Conflicto de concurrencia OCC: el archivo cambió en disco. Hash actual: '{}', esperado: '{}'. Relee el documento antes de escribir.",
                        current_hash, exp
                    ));
                }
            } else if !command.overwrite {
                return Err(format!(
                    "La nota '{}' ya existe (hash '{}', {} líneas). Usa read_note para ver su contenido, patch_note para cambiar una parte, o reenvía con expected_hash (recomendado) u overwrite: true para reemplazarla.",
                    command.relative_path,
                    current_hash,
                    note.content.lines().count()
                ));
            }

            // Freno anti-truncamiento (>150 líneas y reduce más del 40%)
            let old_line_count = note.content.lines().count();
            let new_line_count = content.lines().count();
            if old_line_count > 150
                && !command.allow_large_overwrite
                && new_line_count < (old_line_count * 60) / 100
            {
                return Err(format!(
                    "Freno anti-truncamiento activado: la nota original tiene {} líneas y la nueva tiene {}. Usa patch_note para ediciones parciales o establece allow_large_overwrite: true.",
                    old_line_count, new_line_count
                ));
            }
        }

        let created = existing_note.is_none();
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
                created,
                old_content,
                new_content: final_body,
            });
        }

        let note = Note::with_encoding(
            relative_path,
            "".to_string(),
            title,
            final_body.clone(),
            "UTF-8".to_string(),
        );
        self.note_repository
            .save_note(&command.vault_path, &note)
            .map_err(|err| format!("Error al guardar la nota: {}", err))?;
        Ok(WriteNoteOutcome::Saved {
            created,
            new_hash: NoteContent::hash(&final_body),
        })
    }
}
