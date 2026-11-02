use std::path::Path;

/// Órdenes al sistema operativo, que no controlamos: abrir un archivo con la aplicación por
/// defecto y mostrarlo en el explorador de archivos nativo.
pub trait SystemShell: Send + Sync {
    fn open_with_default_app(&self, vault_path: &Path, relative_path: &str) -> Result<(), String>;
    fn show_in_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String>;
}
