use crate::application::dto::vault_item::list_folder_contents_query::{
    FolderEntryInfo, FolderListing, FolderNoteInfo, ListFolderContentsQuery,
};
use crate::domain::model::note_path::NoteRelativePath;
use crate::domain::repository::note_repository::NoteRepository;
use crate::domain::repository::vault_item_repository::VaultItemRepository;

/// Contenido de una carpeta de la bóveda: subcarpetas y notas.
pub struct ListFolderContentsUseCase<N: NoteRepository, V: VaultItemRepository> {
    notes: N,
    items: V,
}

impl<N: NoteRepository, V: VaultItemRepository> ListFolderContentsUseCase<N, V> {
    pub fn new(notes: N, items: V) -> Self {
        Self { notes, items }
    }

    pub fn execute(&self, query: ListFolderContentsQuery) -> Result<FolderListing, String> {
        let clean_folder = query.folder.as_str();
        if clean_folder.split('/').any(|seg| seg == "..") {
            return Err("Ruta de carpeta no permitida ('..' detectado)".to_string());
        }
        if !self.items.is_folder(&query.vault_path, clean_folder) {
            return Err(format!("La carpeta '{}' no existe o no es un directorio", clean_folder));
        }

        let exclude = query.exclude_note_path.as_deref();
        let mut folders = Vec::new();
        let mut notes = Vec::new();

        if !query.recursive {
            if let Ok(entries) = self.items.list_folder(&query.vault_path, clean_folder) {
                for entry in entries {
                    let name = entry.name;
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        continue;
                    }
                    let rel_path = if clean_folder.is_empty() {
                        name.clone()
                    } else {
                        format!("{}/{}", clean_folder, name)
                    };

                    if entry.is_dir {
                        if query.include_folders {
                            folders.push(FolderEntryInfo { name, relative_path: rel_path });
                        }
                    } else if query.include_notes {
                        if exclude == Some(rel_path.as_str()) {
                            continue;
                        }
                        let line_count = if let Ok(rel) = NoteRelativePath::new(&rel_path) {
                            self.notes.read_note(&query.vault_path, &rel).map(|n| n.content.lines().count()).unwrap_or(0)
                        } else {
                            0
                        };
                        let title = if let Ok(rel) = NoteRelativePath::new(&rel_path) {
                            self.notes.read_note(&query.vault_path, &rel).map(|n| n.title).unwrap_or_else(|_| name.clone())
                        } else {
                            name.clone()
                        };
                        notes.push(FolderNoteInfo {
                            name,
                            title,
                            relative_path: rel_path,
                            line_count: Some(line_count),
                        });
                    }
                }
            }
        } else if query.include_notes {
            let extensions = vec!["md".to_string(), "markdown".to_string()];
            if let Ok(all_notes) = self.notes.list_notes(&query.vault_path, &extensions) {
                for n in all_notes {
                    let path_str = n.relative_path.as_str();
                    if clean_folder.is_empty() || path_str.starts_with(&format!("{}/", clean_folder)) {
                        if exclude == Some(path_str) {
                            continue;
                        }
                        notes.push(FolderNoteInfo {
                            name: path_str.rsplit('/').next().unwrap_or(path_str).to_string(),
                            relative_path: path_str.to_string(),
                            title: n.title.clone(),
                            line_count: None,
                        });
                    }
                }
            }
        }

        Ok(FolderListing { folders, notes })
    }
}
