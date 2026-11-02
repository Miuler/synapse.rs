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
    /// Si existe un archivo o carpeta en la ruta.
    fn exists(&self, vault_path: &Path, relative_path: &str) -> bool;
    /// Si la ruta existe y es una carpeta.
    fn is_folder(&self, vault_path: &Path, relative_path: &str) -> bool;
    /// Archivos dentro de una carpeta y sus subcarpetas (sin ocultos), relativos a esa carpeta.
    fn list_files_recursive(&self, vault_path: &Path, folder: &str) -> Result<Vec<String>, String>;
    /// Todas las carpetas de la bóveda (sin ocultas, `target` ni `node_modules`), relativas a ella.
    fn list_all_folders(&self, vault_path: &Path) -> Result<Vec<String>, String>;
    fn read_text(&self, vault_path: &Path, relative_path: &str) -> Result<String, String>;
}

/// Permite pasar el repositorio por referencia a un caso de uso sin moverlo ni clonarlo.
impl<T: VaultItemRepository + ?Sized> VaultItemRepository for &T {
    fn exists(&self, vault_path: &Path, relative_path: &str) -> bool {
        (**self).exists(vault_path, relative_path)
    }

    fn is_folder(&self, vault_path: &Path, relative_path: &str) -> bool {
        (**self).is_folder(vault_path, relative_path)
    }

    fn list_files_recursive(&self, vault_path: &Path, folder: &str) -> Result<Vec<String>, String> {
        (**self).list_files_recursive(vault_path, folder)
    }

    fn list_all_folders(&self, vault_path: &Path) -> Result<Vec<String>, String> {
        (**self).list_all_folders(vault_path)
    }

    fn delete_item(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        (**self).delete_item(vault_path, relative_path)
    }

    fn rename_item(&self, vault_path: &Path, relative_path: &str, new_name: &str) -> Result<String, String> {
        (**self).rename_item(vault_path, relative_path, new_name)
    }

    fn move_item(&self, vault_path: &Path, source_rel: &str, dest_dir_rel: &str) -> Result<String, String> {
        (**self).move_item(vault_path, source_rel, dest_dir_rel)
    }

    fn copy_item(&self, vault_path: &Path, source_rel: &str, dest_dir_rel: &str) -> Result<String, String> {
        (**self).copy_item(vault_path, source_rel, dest_dir_rel)
    }

    fn write_binary_item(&self, vault_path: &Path, relative_path: &str, bytes: &[u8]) -> Result<(), String> {
        (**self).write_binary_item(vault_path, relative_path, bytes)
    }

    fn read_binary_item(&self, vault_path: &Path, relative_path: &str) -> Result<Vec<u8>, String> {
        (**self).read_binary_item(vault_path, relative_path)
    }

    fn create_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        (**self).create_folder(vault_path, relative_path)
    }

    fn ensure_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        (**self).ensure_folder(vault_path, relative_path)
    }

    fn relocate(&self, vault_path: &Path, source_rel: &str, dest_rel: &str) -> Result<(), String> {
        (**self).relocate(vault_path, source_rel, dest_rel)
    }

    fn list_folder(&self, vault_path: &Path, relative_path: &str) -> Result<Vec<FolderEntry>, String> {
        (**self).list_folder(vault_path, relative_path)
    }

    fn read_text(&self, vault_path: &Path, relative_path: &str) -> Result<String, String> {
        (**self).read_text(vault_path, relative_path)
    }
}
