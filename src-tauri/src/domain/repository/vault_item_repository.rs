use crate::domain::model::folder::FolderEntry;
use std::path::Path;

pub trait VaultItemRepository: Send + Sync {
    fn delete_item(&self, vault_path: &Path, relative_path: &str) -> Result<(), String>;
    fn rename_item(&self, vault_path: &Path, relative_path: &str, new_name: &str) -> Result<String, String>;
    fn move_item(&self, vault_path: &Path, source_rel: &str, dest_dir_rel: &str) -> Result<String, String>;
    fn copy_item(&self, vault_path: &Path, source_rel: &str, dest_dir_rel: &str) -> Result<String, String>;
    fn write_binary_item(&self, vault_path: &Path, relative_path: &str, bytes: &[u8]) -> Result<(), String>;
    fn read_binary_item(&self, vault_path: &Path, relative_path: &str) -> Result<Vec<u8>, String>;
    fn create_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String>;
    /// Crea la carpeta y sus padres si no existen.
    fn ensure_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String>;
    /// Mueve un archivo a una ruta exacta, sobrescribiendo si ya existe.
    fn relocate(&self, vault_path: &Path, source_rel: &str, dest_rel: &str) -> Result<(), String>;
    /// Entradas directas de una carpeta, sin recorrer subcarpetas.
    fn list_folder(&self, vault_path: &Path, relative_path: &str) -> Result<Vec<FolderEntry>, String>;
    fn read_text(&self, vault_path: &Path, relative_path: &str) -> Result<String, String>;
}
