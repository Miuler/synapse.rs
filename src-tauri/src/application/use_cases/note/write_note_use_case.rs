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
            .map_err(|e| format!("Invalid path: {}", e))?;
        let content = command.content.as_str();

        // Leer nota existente si existe
        let existing_note = match self
            .note_repository
            .read_note(&command.vault_path, &relative_path)
        {
            Ok(note) => Some(note),
            Err(e) if e.starts_with("File does not exist") => None,
            // Existe pero no se pudo leer: tratarla como nueva la destruiría.
            Err(e) => return Err(format!("Failed to read existing note: {}", e)),
        };

        if existing_note.is_none() && command.expected_hash.is_some() {
            return Err(format!(
                "expected_hash was specified, but note '{}' does not exist: it may have been moved or deleted. Check the path; to create it, omit expected_hash.",
                command.relative_path
            ));
        }

        if let Some(ref note) = existing_note {
            let current_hash = NoteContent::hash(&note.content);
            if let Some(exp) = command.expected_hash.as_deref() {
                if exp != current_hash {
                    return Err(format!(
                        "OCC concurrency conflict: the file changed on disk. Current hash: '{}', expected: '{}'. Re-read the document before writing.",
                        current_hash, exp
                    ));
                }
            } else if !command.overwrite {
                return Err(format!(
                    "Note '{}' already exists (hash '{}', {} lines). Use read_note to inspect it, patch_note to edit a section, or resend with expected_hash (recommended) or overwrite: true to replace it.",
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
                    "Anti-truncation guard triggered: the original note has {} lines and the new content has {}. Use patch_note for partial edits or set allow_large_overwrite: true.",
                    old_line_count, new_line_count
                ));
            }
        }

        let created = existing_note.is_none();
        let title = command.title.clone().unwrap_or_else(|| {
            existing_note
                .as_ref()
                .map(|n| n.title.clone())
                .unwrap_or_else(|| "Untitled".to_string())
        });

        let final_body = if content.trim_start().starts_with("---") {
            content.to_string()
        } else {
            let mut fm = existing_note
                .as_ref()
                .and_then(|n| n.frontmatter.clone())
                .unwrap_or_default();
            // `title` solo si se pidió (el título por defecto es el nombre del archivo)
            if let Some(t) = command.title.clone() {
                fm.title = Some(t);
            }
            if let Some(tags) = command.tags {
                fm.tags = tags;
            }
            if let Some(aliases) = command.aliases {
                fm.aliases = aliases;
            }
            // Sin nada que poner, sin frontmatter
            if fm.is_empty() {
                content.to_string()
            } else {
                NoteFrontmatter::serialize_with_content(Some(&fm), content)
            }
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
            .map_err(|err| format!("Failed to save note: {}", err))?;
        Ok(WriteNoteOutcome::Saved {
            created,
            new_hash: NoteContent::hash(&final_body),
        })
    }
}
