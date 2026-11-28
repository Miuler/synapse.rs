use crate::application::dto::vault_item::move_folder_contents_command::{
    MoveFolderContentsCommand, MoveFolderContentsResult,
};
use crate::domain::vault::vault_item_repository::VaultItemRepository;
use std::path::Path;

/// Mueve el contenido de una carpeta a otra, con estrategia ante conflictos de nombre.
pub struct MoveFolderContentsUseCase<V: VaultItemRepository> {
    vault_item_repository: V,
}

impl<V: VaultItemRepository> MoveFolderContentsUseCase<V> {
    pub fn new(vault_item_repository: V) -> Self {
        Self {
            vault_item_repository,
        }
    }

    pub fn execute(
        &self,
        command: MoveFolderContentsCommand,
    ) -> Result<MoveFolderContentsResult, String> {
        let vault = &command.vault_path;
        let source_folder = command.source_folder.as_str();
        let target_folder = command.target_folder.as_str();

        if source_folder.split('/').any(|s| s == "..")
            || target_folder.split('/').any(|s| s == "..")
        {
            return Err("Path not allowed ('..' detected)".to_string());
        }
        if source_folder == target_folder {
            return Err("Source and destination folders cannot be the same".to_string());
        }
        // Prevenir bucle infinito: target no puede ser subcarpeta de source
        if target_folder.starts_with(&format!("{}/", source_folder)) {
            return Err(
                "Destination folder cannot be inside the source folder".to_string(),
            );
        }
        if !self.vault_item_repository.is_folder(vault, source_folder) {
            return Err(format!("Source folder '{}' does not exist", source_folder));
        }

        let files_to_move: Vec<String> = self
            .vault_item_repository
            .list_files_recursive(vault, source_folder)
            .unwrap_or_default()
            .into_iter()
            .filter(|rel| match command.extension.as_deref() {
                Some(ext) => rel.to_lowercase().ends_with(&format!(".{}", ext)),
                None => true,
            })
            .collect();

        if command.dry_run {
            return Ok(MoveFolderContentsResult::Preview {
                files: files_to_move,
            });
        }

        self.vault_item_repository
            .ensure_folder(vault, target_folder)
            .map_err(|e| format!("Could not create destination folder: {}", e))?;

        let mut moved = Vec::new();
        let mut skipped = Vec::new();
        let mut errors = Vec::new();

        for rel_file in files_to_move {
            let src_rel = Path::new(source_folder)
                .join(&rel_file)
                .to_string_lossy()
                .to_string();
            let mut dest = Path::new(target_folder).join(&rel_file);

            if let Some(parent) = dest.parent() {
                let _ = self
                    .vault_item_repository
                    .ensure_folder(vault, &parent.to_string_lossy());
            }

            if self
                .vault_item_repository
                .exists(vault, &dest.to_string_lossy())
            {
                match command.conflict_strategy.as_str() {
                    "skip" => {
                        skipped.push(rel_file);
                        continue;
                    }
                    "overwrite" => {}
                    "rename" => {
                        // Generar nombre no colisionante: archivo_1.ext
                        let stem = dest
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("file")
                            .to_string();
                        let ext = dest
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        let parent = dest.parent().unwrap().to_path_buf();
                        let mut counter = 1;
                        loop {
                            let candidate_name = if ext.is_empty() {
                                format!("{}_{}", stem, counter)
                            } else {
                                format!("{}_{}.{}", stem, counter, ext)
                            };
                            let candidate = parent.join(&candidate_name);
                            if !self
                                .vault_item_repository
                                .exists(vault, &candidate.to_string_lossy())
                            {
                                dest = candidate;
                                break;
                            }
                            counter += 1;
                        }
                    }
                    _ => {
                        skipped.push(rel_file);
                        continue;
                    }
                }
            }

            let dest_rel = dest.to_string_lossy().to_string();
            match self
                .vault_item_repository
                .relocate(vault, &src_rel, &dest_rel)
            {
                Ok(_) => moved.push((rel_file, dest_rel.replace('\\', "/"))),
                Err(e) => errors.push(format!("Error moving '{}': {}", rel_file, e)),
            }
        }

        Ok(MoveFolderContentsResult::Done {
            moved,
            skipped,
            errors,
        })
    }
}
