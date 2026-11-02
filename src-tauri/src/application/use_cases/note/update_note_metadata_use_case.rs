use crate::application::dto::note::update_note_metadata_command::UpdateNoteMetadataCommand;
use crate::domain::note::frontmatter::NoteFrontmatter;
use crate::domain::note::note::Note;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;
use std::collections::HashSet;

/// Actualiza el frontmatter de una nota (título, etiquetas, alias) sin tocar el cuerpo.
/// Devuelve el frontmatter resultante y la nueva huella del contenido.
pub struct UpdateNoteMetadataUseCase<N: NoteRepository> {
    notes: N,
}

impl<N: NoteRepository> UpdateNoteMetadataUseCase<N> {
    pub fn new(notes: N) -> Self {
        Self { notes }
    }

    pub fn execute(&self, command: UpdateNoteMetadataCommand) -> Result<(NoteFrontmatter, String), String> {
        let relative_path = NoteRelativePath::new(&command.relative_path).map_err(|e| format!("Ruta inválida: {}", e))?;
        let note = self
            .notes
            .read_note(&command.vault_path, &relative_path)
            .map_err(|e| format!("Error al leer nota: {}", e))?;

        let (fm_opt, body) = NoteFrontmatter::parse(&note.content);
        let mut fm = fm_opt.unwrap_or_default();

        if let Some(new_title) = command.title {
            fm.title = Some(new_title);
        }
        if let Some(add_tags) = command.add_tags {
            let mut tag_set: HashSet<String> = fm.tags.into_iter().collect();
            for t in add_tags {
                tag_set.insert(t);
            }
            fm.tags = tag_set.into_iter().collect();
            fm.tags.sort();
        }
        if let Some(remove_tags) = command.remove_tags {
            let remove_set: HashSet<&str> = remove_tags.iter().map(|t| t.as_str()).collect();
            fm.tags.retain(|t| !remove_set.contains(t.as_str()));
        }
        if let Some(aliases) = command.aliases {
            fm.aliases = aliases;
        }

        let new_full_content = NoteFrontmatter::serialize_with_content(Some(&fm), &body);
        let new_hash = NoteContent::hash(&new_full_content);
        let final_title = fm.title.clone().unwrap_or(note.title);
        let updated_note = Note::with_encoding(relative_path, "".to_string(), final_title, new_full_content, note.encoding);
        self.notes
            .save_note(&command.vault_path, &updated_note)
            .map_err(|err| format!("Error al actualizar metadatos: {}", err))?;
        Ok((fm, new_hash))
    }
}
