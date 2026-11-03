use crate::application::dto::vault_item::move_notes_command::{MoveNotesCommand, MoveNotesResult};
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Mueve notas a una carpeta y, si se pide, reescribe los enlaces que apuntaban a ellas.
pub struct MoveNotesUseCase<N: NoteRepository, V: VaultItemRepository> {
    note_repository: N,
    vault_item_repository: V,
}

impl<N: NoteRepository, V: VaultItemRepository> MoveNotesUseCase<N, V> {
    pub fn new(note_repository: N, vault_item_repository: V) -> Self {
        Self {
            note_repository,
            vault_item_repository,
        }
    }

    pub fn execute(&self, command: MoveNotesCommand) -> Result<MoveNotesResult, String> {
        let vault = &command.vault_path;
        let clean_target = command.target_folder.trim_matches('/').replace('\\', "/");
        if clean_target.split('/').any(|seg| seg == "..") {
            return Err("Ruta de destino no permitida ('..' detectado)".to_string());
        }

        if !self.vault_item_repository.exists(vault, &clean_target) {
            if command.create_target_folder {
                self.vault_item_repository
                    .ensure_folder(vault, &clean_target)
                    .map_err(|e| format!("Error al crear carpeta de destino: {}", e))?;
            } else {
                return Err(format!("La carpeta destino '{}' no existe", clean_target));
            }
        }

        let mut moved = Vec::new();
        let mut errors = Vec::new();

        for source_rel in &command.notes {
            let clean_src = source_rel.trim_matches('/').replace('\\', "/");
            let file_name = match clean_src.rsplit('/').next() {
                Some(f) => f,
                None => {
                    errors.push((source_rel.clone(), "Ruta inválida".to_string()));
                    continue;
                }
            };
            let target_rel = if clean_target.is_empty() {
                file_name.to_string()
            } else {
                format!("{}/{}", clean_target, file_name)
            };

            if !self.vault_item_repository.exists(vault, &clean_src) {
                errors.push((source_rel.clone(), "Archivo origen no existe".to_string()));
                continue;
            }
            if self.vault_item_repository.exists(vault, &target_rel) && !command.overwrite {
                errors.push((
                    source_rel.clone(),
                    "Ya existe un archivo en destino".to_string(),
                ));
                continue;
            }

            match self
                .vault_item_repository
                .relocate(vault, &clean_src, &target_rel)
            {
                Ok(_) => {
                    if command.update_wikilinks {
                        self.rewrite_wikilinks(vault, &clean_src, file_name, &target_rel);
                    }
                    moved.push((clean_src.clone(), target_rel));
                }
                Err(e) => errors.push((source_rel.clone(), e)),
            }
        }

        Ok(MoveNotesResult { moved, errors })
    }

    /// Reescritura simple de `[[enlaces]]` hacia la nota movida.
    fn rewrite_wikilinks(
        &self,
        vault: &std::path::Path,
        clean_src: &str,
        file_name: &str,
        target_rel: &str,
    ) {
        let old_stem = file_name
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(file_name);
        let extensions = vec!["md".to_string(), "markdown".to_string()];
        if let Ok(all_notes) = self.note_repository.list_notes(vault, &extensions) {
            for other in all_notes {
                if other.relative_path.as_str() == target_rel {
                    continue;
                }
                if let Ok(mut note_to_patch) =
                    self.note_repository.read_note(vault, &other.relative_path)
                {
                    if note_to_patch.content.contains(&format!("[[{}]]", old_stem))
                        || note_to_patch
                            .content
                            .contains(&format!("[[{}]]", clean_src))
                    {
                        note_to_patch.content = note_to_patch
                            .content
                            .replace(
                                &format!("[[{}]]", clean_src),
                                &format!("[[{}]]", target_rel),
                            )
                            .replace(&format!("[[{}]]", old_stem), &format!("[[{}]]", target_rel));
                        let _ = self.note_repository.save_note(vault, &note_to_patch);
                    }
                }
            }
        }
    }
}
