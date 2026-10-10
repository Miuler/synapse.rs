use crate::domain::services::file_system_service::FileSystemPort;
use std::path::Path;

/// Casos de uso de la aplicación para operaciones de sistema de archivos.
pub struct FileSystemUseCases<F: FileSystemPort> {
    port: F,
}

impl<F: FileSystemPort> FileSystemUseCases<F> {
    pub fn new(port: F) -> Self {
        Self { port }
    }

    pub fn delete_item(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        self.port.delete_item(vault_path, relative_path)
    }

    pub fn rename_item(&self, vault_path: &Path, relative_path: &str, new_name: &str) -> Result<String, String> {
        self.port.rename_item(vault_path, relative_path, new_name)
    }

    pub fn copy_items(&self, vault_path: &Path, paths: &[String], dest_dir: &str) -> Result<Vec<String>, String> {
        let mut created = Vec::with_capacity(paths.len());
        for source in paths {
            created.push(self.port.copy_item(vault_path, source, dest_dir)?);
        }
        Ok(created)
    }
}
