use crate::domain::services::copy_naming::next_available_copy_name;
use std::fs;
use std::path::{Path, PathBuf};

/// Servicio de infraestructura para operaciones de sistema de archivos dentro de la bóveda.
pub struct FileSystemService;

impl FileSystemService {
    /// Resuelve una ruta relativa de la bóveda a una ruta absoluta segura (sin `..`).
    fn resolve(vault_path: &Path, relative_path: &str) -> Result<PathBuf, String> {
        let mut target = vault_path.to_path_buf();
        for part in relative_path.replace('\\', "/").split('/') {
            if part.is_empty() || part == "." {
                continue;
            }
            if part == ".." {
                return Err("Ruta no permitida con '..'".to_string());
            }
            target.push(part);
        }
        Ok(target)
    }

    fn to_relative(vault_path: &Path, abs: &Path) -> String {
        abs.strip_prefix(vault_path)
            .unwrap_or(abs)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let from = entry.path();
            let to = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                Self::copy_dir_recursive(&from, &to)?;
            } else {
                fs::copy(&from, &to)?;
            }
        }
        Ok(())
    }

    /// Copia un archivo o carpeta (`source_rel`) dentro de la carpeta destino (`dest_dir_rel`).
    ///
    /// Si el destino es la misma carpeta de origen, o ya existe un elemento con ese nombre,
    /// se agrega `.copy` al nombre. Retorna la ruta relativa del nuevo elemento.
    pub fn copy_item(vault_path: &Path, source_rel: &str, dest_dir_rel: &str) -> Result<String, String> {
        let source = Self::resolve(vault_path, source_rel)?;
        let dest_dir = Self::resolve(vault_path, dest_dir_rel)?;

        if source == vault_path {
            return Err("No se puede copiar la raíz de la bóveda".to_string());
        }
        if !source.exists() {
            return Err(format!("El elemento '{}' no existe", source_rel));
        }
        if !dest_dir.is_dir() {
            return Err(format!("La carpeta destino '{}' no existe", dest_dir_rel));
        }

        let canonical_vault = vault_path.canonicalize().map_err(|e| e.to_string())?;
        let canonical_source = source.canonicalize().map_err(|e| e.to_string())?;
        let canonical_dest = dest_dir.canonicalize().map_err(|e| e.to_string())?;
        if !canonical_source.starts_with(&canonical_vault) || !canonical_dest.starts_with(&canonical_vault) {
            return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
        }

        let is_folder = canonical_source.is_dir();
        if is_folder && canonical_dest.starts_with(&canonical_source) {
            return Err("No se puede copiar una carpeta dentro de sí misma".to_string());
        }

        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| "Nombre de elemento inválido".to_string())?;

        let same_folder = canonical_source.parent() == Some(canonical_dest.as_path());
        let final_name = if same_folder || dest_dir.join(&name).exists() {
            next_available_copy_name(&name, is_folder, |n| dest_dir.join(n).exists())
        } else {
            name
        };
        let target = dest_dir.join(&final_name);

        if is_folder {
            Self::copy_dir_recursive(&canonical_source, &target)
                .map_err(|e| format!("Error al copiar carpeta: {}", e))?;
        } else {
            fs::copy(&canonical_source, &target).map_err(|e| format!("Error al copiar archivo: {}", e))?;
        }

        Ok(Self::to_relative(vault_path, &target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("synapse_fs_test_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("nota.md"), "hola").unwrap();
        dir
    }

    #[test]
    fn copy_same_folder_appends_copy() {
        let vault = temp_vault("same");
        let rel = FileSystemService::copy_item(&vault, "nota.md", "").unwrap();
        assert_eq!(rel, "nota.copy.md");
        let rel2 = FileSystemService::copy_item(&vault, "nota.md", "").unwrap();
        assert_eq!(rel2, "nota.copy.copy.md");
        let _ = fs::remove_dir_all(&vault);
    }

    #[test]
    fn copy_other_folder_keeps_name() {
        let vault = temp_vault("other");
        let rel = FileSystemService::copy_item(&vault, "nota.md", "sub").unwrap();
        assert_eq!(rel, "sub/nota.md");
        let _ = fs::remove_dir_all(&vault);
    }

    #[test]
    fn copy_folder_into_itself_fails() {
        let vault = temp_vault("self");
        assert!(FileSystemService::copy_item(&vault, "sub", "sub").is_err());
        let rel = FileSystemService::copy_item(&vault, "sub", "").unwrap();
        assert_eq!(rel, "sub.copy");
        let _ = fs::remove_dir_all(&vault);
    }
}
