use std::path::Path;

pub trait FileSystemPort: Send + Sync {
    fn delete_item(&self, vault_path: &Path, relative_path: &str) -> Result<(), String>;
    fn rename_item(&self, vault_path: &Path, relative_path: &str, new_name: &str) -> Result<String, String>;
    fn copy_item(&self, vault_path: &Path, source_rel: &str, dest_dir_rel: &str) -> Result<String, String>;
}
