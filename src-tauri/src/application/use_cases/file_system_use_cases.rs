use crate::domain::repository::vault_item_repository::VaultItemRepository;
use std::path::Path;

/// Casos de uso de la aplicación para operaciones de sistema de archivos.
pub struct FileSystemUseCases<F: VaultItemRepository> {
    repository: F,
}

impl<F: VaultItemRepository> FileSystemUseCases<F> {
    pub fn new(repository: F) -> Self {
        Self { repository }
    }

    pub fn delete_item(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        self.repository.delete_item(vault_path, relative_path)
    }

    pub fn rename_item(&self, vault_path: &Path, relative_path: &str, new_name: &str) -> Result<String, String> {
        self.repository.rename_item(vault_path, relative_path, new_name)
    }

    pub fn move_item(&self, vault_path: &Path, source_rel: &str, dest_dir_rel: &str) -> Result<String, String> {
        self.repository.move_item(vault_path, source_rel, dest_dir_rel)
    }

    pub fn copy_items(&self, vault_path: &Path, paths: &[String], dest_dir: &str) -> Result<Vec<String>, String> {
        let mut created = Vec::with_capacity(paths.len());
        for source in paths {
            created.push(self.repository.copy_item(vault_path, source, dest_dir)?);
        }
        Ok(created)
    }

    pub fn write_binary_item(&self, vault_path: &Path, relative_path: &str, bytes: &[u8]) -> Result<(), String> {
        self.repository.write_binary_item(vault_path, relative_path, bytes)
    }

    pub fn read_binary_item(&self, vault_path: &Path, relative_path: &str) -> Result<Vec<u8>, String> {
        self.repository.read_binary_item(vault_path, relative_path)
    }

    pub fn create_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        self.repository.create_folder(vault_path, relative_path)
    }

    pub fn open_file_with_default_app(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        self.repository.open_file_with_default_app(vault_path, relative_path)
    }

    pub fn show_item_in_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        self.repository.show_item_in_folder(vault_path, relative_path)
    }
}
