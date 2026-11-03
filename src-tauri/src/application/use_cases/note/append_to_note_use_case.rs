use crate::application::dto::note::append_to_note_command::AppendToNoteCommand;
use crate::domain::note::note::Note;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;

/// Anexa contenido al final de una nota o al inicio/final de una sección. Devuelve la nueva
/// huella del contenido.
pub struct AppendToNoteUseCase<N: NoteRepository> {
    note_repository: N,
}

impl<N: NoteRepository> AppendToNoteUseCase<N> {
    pub fn new(note_repository: N) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, command: AppendToNoteCommand) -> Result<String, String> {
        let relative_path = NoteRelativePath::new(&command.relative_path)
            .map_err(|e| format!("Ruta inválida: {}", e))?;
        let note = self
            .note_repository
            .read_note(&command.vault_path, &relative_path)
            .map_err(|e| format!("Error al leer nota: {}", e))?;

        let mut lines: Vec<String> = note.content.lines().map(|s| s.to_string()).collect();

        if let Some(heading) = command.section_heading.as_deref() {
            let target_h = heading.trim().trim_start_matches('#').trim().to_lowercase();
            let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            let (start_h, sec_level) = NoteContent::find_heading(&line_refs, &target_h)
                .ok_or_else(|| format!("Encabezado de sección '{}' no encontrado", heading))?;
            let insert_idx = if command.position == "start_of_section" {
                start_h + 1
            } else {
                NoteContent::section_end(&line_refs, start_h, sec_level)
            };
            lines.insert(insert_idx, command.content);
        } else {
            lines.push(command.content);
        }

        let new_content = lines.join("\n");
        let new_hash = NoteContent::hash(&new_content);
        let updated_note = Note::with_encoding(
            relative_path,
            "".to_string(),
            note.title,
            new_content,
            note.encoding,
        );
        self.note_repository
            .save_note(&command.vault_path, &updated_note)
            .map_err(|err| format!("Error al anexar contenido: {}", err))?;
        Ok(new_hash)
    }
}
