use crate::application::dto::vault_item::search_folders_query::{
    FolderSearchResult, SearchFoldersQuery,
};
use crate::domain::graph::tag::Tag;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::service::folder_summarizer::FolderSummarizer;
use crate::domain::vault::folder::FolderPath;
use crate::domain::vault::vault_item_repository::VaultItemRepository;
use std::collections::HashSet;

/// Busca carpetas por nombre (exactas primero, luego por prefijo) y las resume a partir de sus
/// notas.
pub struct SearchFoldersUseCase<N: NoteRepository, V: VaultItemRepository> {
    note_repository: N,
    vault_item_repository: V,
}

impl<N: NoteRepository, V: VaultItemRepository> SearchFoldersUseCase<N, V> {
    pub fn new(note_repository: N, vault_item_repository: V) -> Self {
        Self {
            note_repository,
            vault_item_repository,
        }
    }

    pub fn execute(&self, query: SearchFoldersQuery) -> Result<FolderSearchResult, String> {
        let extensions = vec![
            "md".to_string(),
            "markdown".to_string(),
            "pdf".to_string(),
            "excalidraw".to_string(),
        ];
        let all_notes = self
            .note_repository
            .list_notes(&query.vault_path, &extensions)
            .map_err(|e| format!("Error leyendo notas para indexar carpetas: {}", e))?;

        // Recolectar todas las carpetas a partir de las notas y del sistema de archivos
        let file_paths: Vec<String> = all_notes
            .iter()
            .map(|n| n.relative_path.as_str().to_string())
            .collect();
        let mut folder_set: HashSet<String> =
            FolderPath::ancestors_of(&file_paths).into_iter().collect();
        // Incluir también carpetas vacías
        if let Ok(folders) = self
            .vault_item_repository
            .list_all_folders(&query.vault_path)
        {
            folder_set.extend(folders);
        }
        let all_folders: Vec<String> = folder_set.into_iter().collect();

        let all_files_vec: Vec<(String, String, Vec<String>)> = all_notes
            .iter()
            .map(|n| {
                (
                    n.relative_path.as_str().to_string(),
                    n.title.clone(),
                    Tag::extract_all(&n.content),
                )
            })
            .collect();
        let files_ref: Vec<(&str, &str, &[String])> = all_files_vec
            .iter()
            .map(|(p, t, tags)| (p.as_str(), t.as_str(), tags.as_slice()))
            .collect();

        let text = query.query.as_str();
        let mut matched: Vec<String> = if text.is_empty() {
            all_folders.clone()
        } else {
            all_folders
                .iter()
                .filter(|f| f.to_lowercase().contains(text))
                .cloned()
                .collect()
        };

        if !text.is_empty() {
            matched.sort_by(|a, b| {
                let name_a = a.rsplit('/').next().unwrap_or(a).to_lowercase();
                let name_b = b.rsplit('/').next().unwrap_or(b).to_lowercase();
                let exact_a = name_a == text;
                let exact_b = name_b == text;
                if exact_a != exact_b {
                    return if exact_a {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    };
                }
                let prefix_a = name_a.starts_with(text);
                let prefix_b = name_b.starts_with(text);
                if prefix_a != prefix_b {
                    return if prefix_a {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    };
                }
                a.cmp(b)
            });
        } else {
            matched.sort();
        }

        let total = matched.len();
        let folders = matched
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .map(|f| FolderSummarizer::summarize(&f, &all_folders, &files_ref))
            .collect();
        Ok(FolderSearchResult { total, folders })
    }
}
