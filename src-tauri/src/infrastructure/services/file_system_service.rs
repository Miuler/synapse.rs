use crate::domain::services::copy_naming::next_available_copy_name;
use std::fs;
use std::path::{Path, PathBuf};

/// Servicio de infraestructura para operaciones de sistema de archivos dentro de la bóveda.
pub struct FileSystemService;

impl FileSystemService {
    /// Resuelve una ruta relativa o absoluta segura dentro de los límites de la bóveda (sin `..`).
    fn resolve(vault_path: &Path, relative_path: &str) -> Result<PathBuf, String> {
        let path_obj = Path::new(relative_path);
        let rel_str = if path_obj.is_absolute() {
            if let Ok(rel) = path_obj.strip_prefix(vault_path) {
                rel.to_string_lossy().to_string()
            } else {
                return Err("Operación no permitida: la ruta está fuera de los límites de la bóveda".to_string());
            }
        } else {
            relative_path.to_string()
        };

        let mut target = vault_path.to_path_buf();
        for part in rel_str.replace('\\', "/").split('/') {
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

    /// Renombra un elemento sin moverlo fuera de su carpeta actual.
    pub fn rename_item(vault_path: &Path, source_rel: &str, new_name: &str) -> Result<String, String> {
        let new_name = new_name.trim();
        if new_name.is_empty() || new_name == "." || new_name == ".." {
            return Err("El nombre no puede estar vacío".to_string());
        }
        if new_name.contains('/') || new_name.contains('\\') {
            return Err("El nombre no puede contener separadores de ruta".to_string());
        }

        let source = Self::resolve(vault_path, source_rel)?;
        if source == vault_path || !source.exists() {
            return Err(format!("El elemento '{}' no existe", source_rel));
        }

        let canonical_vault = vault_path.canonicalize().map_err(|e| e.to_string())?;
        let canonical_source = source.canonicalize().map_err(|e| e.to_string())?;
        if !canonical_source.starts_with(&canonical_vault) {
            return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
        }

        let parent = source.parent().ok_or_else(|| "Elemento sin carpeta padre".to_string())?;
        let target = parent.join(new_name);
        if target == source {
            return Ok(Self::to_relative(vault_path, &source));
        }
        if target.exists() {
            return Err(format!("Ya existe un elemento llamado '{}'", new_name));
        }

        fs::rename(&source, &target).map_err(|e| format!("Error al renombrar elemento: {}", e))?;
        Ok(Self::to_relative(vault_path, &target))
    }

    /// Elimina un archivo o carpeta dentro de la bóveda garantizando los límites de seguridad.
    pub fn delete_item(vault_path: &Path, relative_path: &str) -> Result<(), String> {
        let clean_rel = relative_path.replace('\\', "/");
        let mut target_path = vault_path.to_path_buf();
        for part in clean_rel.split('/') {
            if part.is_empty() || part == "." {
                continue;
            }
            if part == ".." {
                return Err("Ruta no permitida con '..'".to_string());
            }
            target_path.push(part);
        }

        if target_path == vault_path {
            return Err("No se puede eliminar la raíz de la bóveda".to_string());
        }

        if !target_path.exists() {
            return Err(format!("El elemento '{}' no existe", relative_path));
        }

        let canonical_vault = vault_path.canonicalize().map_err(|e| e.to_string())?;
        let canonical_target = target_path.canonicalize().map_err(|e| e.to_string())?;
        if !canonical_target.starts_with(&canonical_vault) || canonical_target == canonical_vault {
            return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
        }

        if canonical_target.is_dir() {
            std::fs::remove_dir_all(&canonical_target)
                .map_err(|e| format!("Error al eliminar carpeta: {}", e))?;
        } else if canonical_target.is_file() {
            std::fs::remove_file(&canonical_target)
                .map_err(|e| format!("Error al eliminar archivo: {}", e))?;
        } else {
            return Err("Tipo de elemento no soportado para eliminar".to_string());
        }

        Ok(())
    }

    /// Escribe contenido binario en un archivo dentro de la bóveda garantizando los límites de seguridad.
    pub fn write_binary_item(vault_path: &Path, relative_path: &str, bytes: &[u8]) -> Result<(), String> {
        let target = Self::resolve(vault_path, relative_path)?;
        if target == vault_path {
            return Err("No se puede escribir directamente en la raíz de la bóveda".to_string());
        }

        let canonical_vault = vault_path.canonicalize().map_err(|e| e.to_string())?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Error al crear directorio: {}", e))?;
            let canonical_parent = parent.canonicalize().map_err(|e| e.to_string())?;
            if !canonical_parent.starts_with(&canonical_vault) {
                return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
            }
        }

        fs::write(&target, bytes).map_err(|e| format!("Error al escribir archivo binario: {}", e))?;
        Ok(())
    }

    /// Lee contenido binario de un archivo dentro de la bóveda garantizando los límites de seguridad.
    pub fn read_binary_item(vault_path: &Path, relative_path: &str) -> Result<Vec<u8>, String> {
        let target = Self::resolve(vault_path, relative_path)?;
        if !target.is_file() {
            return Err(format!("El archivo no existe o no es un archivo: {}", relative_path));
        }

        fs::read(&target).map_err(|e| format!("Error al leer archivo binario: {}", e))
    }

    /// Crea una carpeta dentro de la bóveda garantizando los límites de seguridad.
    pub fn create_folder(vault_path: &Path, relative_path: &str) -> Result<(), String> {
        let target = Self::resolve(vault_path, relative_path)?;
        fs::create_dir_all(&target).map_err(|e| format!("Error al crear carpeta: {}", e))
    }

    /// Abre un archivo con la aplicación por defecto del sistema operativo (Word, Excel, etc.).
    pub fn open_file_with_default_app(vault_path: &Path, relative_path: &str) -> Result<(), String> {
        let abs_path = if Path::new(relative_path).is_absolute() {
            let p = PathBuf::from(relative_path);
            if !p.exists() {
                return Err(format!("El archivo no existe: {}", p.display()));
            }
            p
        } else {
            Self::resolve(vault_path, relative_path)?
        };

        if !abs_path.exists() {
            return Err(format!("El archivo no existe: {}", abs_path.display()));
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", &abs_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Error al abrir archivo en Windows: {}", e))?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&abs_path)
                .spawn()
                .map_err(|e| format!("Error al abrir archivo en macOS: {}", e))?;
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&abs_path)
                .spawn()
                .map_err(|e| format!("Error al abrir archivo con xdg-open: {}", e))?;
        }

        Ok(())
    }

    /// Muestra y resalta un archivo o carpeta en el explorador de archivos nativo del sistema.
    pub fn show_item_in_folder(vault_path: &Path, relative_path: &str) -> Result<(), String> {
        let abs_path = if Path::new(relative_path).is_absolute() {
            let p = PathBuf::from(relative_path);
            if !p.exists() {
                return Err(format!("La ruta no existe: {}", p.display()));
            }
            p
        } else {
            Self::resolve(vault_path, relative_path)?
        };

        if !abs_path.exists() {
            return Err(format!("La ruta no existe: {}", abs_path.display()));
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(format!("/select,\"{}\"", abs_path.to_string_lossy()))
                .spawn()
                .map_err(|e| format!("Error al mostrar en carpeta en Windows: {}", e))?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .args(["-R", &abs_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Error al mostrar en carpeta en macOS: {}", e))?;
        }

        #[cfg(target_os = "linux")]
        {
            let mut opened = false;

            // 1. Estándar FreeDesktop: D-Bus org.freedesktop.FileManager1.ShowItems
            // Es la especificación oficial soportada por Dolphin (KDE), Nautilus (GNOME), Nemo (Cinnamon), Thunar (XFCE), PCManFM-Qt (LXQt), etc.
            let uri = format!("file://{}", abs_path.to_string_lossy());

            // 1a. Intentar con busctl (systemd / moderno)
            if let Ok(mut child) = std::process::Command::new("busctl")
                .args([
                    "--user",
                    "call",
                    "org.freedesktop.FileManager1",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1",
                    "ShowItems",
                    "ass",
                    "1",
                    &uri,
                    "",
                ])
                .spawn()
            {
                if let Ok(status) = child.wait() {
                    if status.success() {
                        opened = true;
                    }
                }
            }

            // 1b. Si busctl falló o no está, intentar con dbus-send
            if !opened {
                if let Ok(mut child) = std::process::Command::new("dbus-send")
                    .args([
                        "--session",
                        "--dest=org.freedesktop.FileManager1",
                        "--type=method_call",
                        "/org/freedesktop/FileManager1",
                        "org.freedesktop.FileManager1.ShowItems",
                        &format!("array:string:\"{}\"", uri),
                        "string:\"\"",
                    ])
                    .spawn()
                {
                    if let Ok(status) = child.wait() {
                        if status.success() {
                            opened = true;
                        }
                    }
                }
            }

            // 2. Fallbacks directos por CLI si D-Bus no respondió o no está presente el servicio
            if !opened {
                let fallback_commands = [
                    ("dolphin", vec!["--select".to_string(), abs_path.to_string_lossy().to_string()]),
                    ("nautilus", vec!["--select".to_string(), abs_path.to_string_lossy().to_string()]),
                    ("nemo", vec!["--no-desktop".to_string(), abs_path.to_string_lossy().to_string()]),
                    ("thunar", vec![abs_path.to_string_lossy().to_string()]),
                    ("pcmanfm", vec![abs_path.to_string_lossy().to_string()]),
                    ("pcmanfm-qt", vec![abs_path.to_string_lossy().to_string()]),
                ];

                for (cmd, args) in fallback_commands {
                    if let Ok(_) = std::process::Command::new(cmd).args(&args).spawn() {
                        opened = true;
                        break;
                    }
                }
            }

            if !opened {
                // Último fallback: abrir la carpeta contenedora con xdg-open
                let target_dir = if abs_path.is_dir() {
                    abs_path.clone()
                } else {
                    abs_path.parent().unwrap_or(&abs_path).to_path_buf()
                };
                std::process::Command::new("xdg-open")
                    .arg(&target_dir)
                    .spawn()
                    .map_err(|e| format!("Error al abrir carpeta contenedora con xdg-open: {}", e))?;
            }
        }

        Ok(())
    }
}

use crate::domain::services::file_system_service::FileSystemPort;

impl FileSystemPort for FileSystemService {
    fn delete_item(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        Self::delete_item(vault_path, relative_path)
    }

    fn rename_item(&self, vault_path: &Path, relative_path: &str, new_name: &str) -> Result<String, String> {
        Self::rename_item(vault_path, relative_path, new_name)
    }

    fn copy_item(&self, vault_path: &Path, source_rel: &str, dest_dir_rel: &str) -> Result<String, String> {
        Self::copy_item(vault_path, source_rel, dest_dir_rel)
    }

    fn write_binary_item(&self, vault_path: &Path, relative_path: &str, bytes: &[u8]) -> Result<(), String> {
        Self::write_binary_item(vault_path, relative_path, bytes)
    }

    fn read_binary_item(&self, vault_path: &Path, relative_path: &str) -> Result<Vec<u8>, String> {
        Self::read_binary_item(vault_path, relative_path)
    }

    fn create_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        Self::create_folder(vault_path, relative_path)
    }

    fn open_file_with_default_app(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        Self::open_file_with_default_app(vault_path, relative_path)
    }

    fn show_item_in_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        Self::show_item_in_folder(vault_path, relative_path)
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

    #[test]
    fn write_and_read_binary_item() {
        let vault = temp_vault("binary");
        let pdf_bytes = b"%PDF-1.7\nsome binary content\n%%EOF";
        let res = FileSystemService::write_binary_item(&vault, "docs/sample.pdf", pdf_bytes);
        assert!(res.is_ok());

        let read_bytes = fs::read(vault.join("docs/sample.pdf")).unwrap();
        assert_eq!(read_bytes, pdf_bytes);

        // Test read_binary_item directly
        let direct_read = FileSystemService::read_binary_item(&vault, "docs/sample.pdf").unwrap();
        assert_eq!(direct_read, pdf_bytes);

        // Test create_folder
        let folder_res = FileSystemService::create_folder(&vault, "archivos/adjuntos/excalidraw");
        assert!(folder_res.is_ok());
        assert!(vault.join("archivos/adjuntos/excalidraw").is_dir());

        let _ = fs::remove_dir_all(&vault);
    }
}
