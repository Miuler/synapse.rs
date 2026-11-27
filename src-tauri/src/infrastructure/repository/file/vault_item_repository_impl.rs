use crate::domain::vault::file_name::FileName;
use crate::domain::board::protection;
use crate::infrastructure::repository::file::board_guard;
use std::fs;
use std::path::{Path, PathBuf};

/// Servicio de infraestructura para operaciones de sistema de archivos dentro de la bóveda.
#[derive(Clone)]
pub struct VaultItemRepositoryImpl;

/// La carpeta de tareas que acompaña a un archivo de tablero, si existe.
fn board_tasks(board: &Path) -> Option<PathBuf> {
    protection::folder_of(board).filter(|folder| board.is_file() && folder.is_dir())
}

/// Dónde iría la carpeta de un tablero que pasa a llamarse `target`: libre, y sin dejar de ser tablero.
fn free_board_folder(target: &Path) -> Result<PathBuf, String> {
    let folder = protection::folder_of(target).ok_or_else(|| {
        "Un tablero con carpeta de tareas conserva la extensión «.board.md»".to_string()
    })?;
    if folder.exists() {
        return Err(format!(
            "Ya existe «{}»",
            folder.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    Ok(folder)
}

/// Renombra o mueve; si es un tablero con carpeta, la carpeta va con él.
fn rename_with_board(source: &Path, target: &Path) -> Result<(), String> {
    let Some(tasks) = board_tasks(source) else {
        return fs::rename(source, target).map_err(|e| e.to_string());
    };
    let tasks_target = free_board_folder(target)?;
    fs::rename(source, target).map_err(|e| e.to_string())?;
    fs::rename(&tasks, &tasks_target).map_err(|e| {
        // Separados, la carpeta dejaría de estar protegida: se deshace
        let _ = fs::rename(target, source);
        e.to_string()
    })
}

impl VaultItemRepositoryImpl {
    /// Resuelve una ruta relativa o absoluta segura dentro de los límites de la bóveda (sin `..`).
    pub(crate) fn resolve(vault_path: &Path, relative_path: &str) -> Result<PathBuf, String> {
        let path_obj = Path::new(relative_path);
        let rel_str = if path_obj.is_absolute() {
            if let Ok(rel) = path_obj.strip_prefix(vault_path) {
                rel.to_string_lossy().to_string()
            } else {
                return Err(
                    "Operación no permitida: la ruta está fuera de los límites de la bóveda"
                        .to_string(),
                );
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
    pub fn copy_item(
        vault_path: &Path,
        source_rel: &str,
        dest_dir_rel: &str,
    ) -> Result<String, String> {
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
        if !canonical_source.starts_with(&canonical_vault)
            || !canonical_dest.starts_with(&canonical_vault)
        {
            return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
        }

        Self::copy_into(vault_path, &canonical_source, &dest_dir, &canonical_dest)
    }

    /// Copia a la carpeta destino de la bóveda un archivo o carpeta de fuera (`source_abs`, ruta
    /// absoluta: p. ej. lo copiado en otra instancia o en el explorador de archivos del sistema).
    pub fn import_item(
        vault_path: &Path,
        source_abs: &Path,
        dest_dir_rel: &str,
    ) -> Result<String, String> {
        let dest_dir = Self::resolve(vault_path, dest_dir_rel)?;
        if !source_abs.is_absolute() || !source_abs.exists() {
            return Err(format!("El elemento '{}' no existe", source_abs.display()));
        }
        if !dest_dir.is_dir() {
            return Err(format!("La carpeta destino '{}' no existe", dest_dir_rel));
        }
        let canonical_vault = vault_path.canonicalize().map_err(|e| e.to_string())?;
        let canonical_source = source_abs.canonicalize().map_err(|e| e.to_string())?;
        let canonical_dest = dest_dir.canonicalize().map_err(|e| e.to_string())?;
        if !canonical_dest.starts_with(&canonical_vault) {
            return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
        }
        Self::copy_into(vault_path, &canonical_source, &dest_dir, &canonical_dest)
    }

    /// Copia `canonical_source` dentro de `dest_dir` sin pisar nada (si el nombre existe, «copia»).
    fn copy_into(
        vault_path: &Path,
        canonical_source: &Path,
        dest_dir: &Path,
        canonical_dest: &Path,
    ) -> Result<String, String> {
        let is_folder = canonical_source.is_dir();
        if is_folder && canonical_dest.starts_with(canonical_source) {
            return Err("No se puede copiar una carpeta dentro de sí misma".to_string());
        }

        let name = canonical_source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| "Nombre de elemento inválido".to_string())?;

        let same_folder = canonical_source.parent() == Some(canonical_dest);
        let board = board_tasks(canonical_source).and(name.strip_suffix(".board.md"));
        let final_name = if let Some(stem) = board {
            // La copia de un tablero se numera antes de «.board.md», y exige libre también su carpeta
            let taken = |n: &str| {
                let stem = n.strip_suffix(".md").unwrap_or(n);
                dest_dir.join(format!("{stem}.board.md")).exists()
                    || dest_dir.join(format!("{stem}.board")).exists()
            };
            let plain = format!("{stem}.md");
            let free = if same_folder || taken(&plain) {
                FileName::next_available_copy_name(&plain, false, taken)
            } else {
                plain
            };
            format!("{}.board.md", free.strip_suffix(".md").unwrap_or(&free))
        } else if same_folder || dest_dir.join(&name).exists() {
            FileName::next_available_copy_name(&name, is_folder, |n| dest_dir.join(n).exists())
        } else {
            name
        };
        let target = dest_dir.join(&final_name);
        board_guard::ensure_writable(&target)?;
        // La carpeta de un tablero copiado va con él, así que su destino también tiene que estar libre
        let tasks = board_tasks(canonical_source);
        let tasks_target = match &tasks {
            Some(_) => Some(free_board_folder(&target)?),
            None => None,
        };

        if is_folder {
            Self::copy_dir_recursive(canonical_source, &target)
                .map_err(|e| format!("Error al copiar carpeta: {}", e))?;
        } else {
            fs::copy(canonical_source, &target)
                .map_err(|e| format!("Error al copiar archivo: {}", e))?;
        }
        if let (Some(tasks), Some(tasks_target)) = (tasks, tasks_target) {
            Self::copy_dir_recursive(&tasks, &tasks_target)
                .map_err(|e| format!("Error al copiar la carpeta del tablero: {}", e))?;
        }

        Ok(Self::to_relative(vault_path, &target))
    }

    /// Mueve un archivo o carpeta (`source_rel`) dentro de la carpeta destino (`dest_dir_rel`).
    ///
    /// No sobrescribe: si ya existe un elemento con ese nombre en el destino devuelve error.
    /// Mover al mismo lugar es un no-op. Retorna la nueva ruta relativa.
    pub fn move_item(
        vault_path: &Path,
        source_rel: &str,
        dest_dir_rel: &str,
    ) -> Result<String, String> {
        let source = Self::resolve(vault_path, source_rel)?;
        let dest_dir = Self::resolve(vault_path, dest_dir_rel)?;

        if source == vault_path {
            return Err("No se puede mover la raíz de la bóveda".to_string());
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
        if !canonical_source.starts_with(&canonical_vault)
            || !canonical_dest.starts_with(&canonical_vault)
        {
            return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
        }
        if canonical_source.is_dir() && canonical_dest.starts_with(&canonical_source) {
            return Err("No se puede mover una carpeta dentro de sí misma".to_string());
        }

        let name = source
            .file_name()
            .map(|n| n.to_os_string())
            .ok_or_else(|| "Nombre de elemento inválido".to_string())?;
        if canonical_source.parent() == Some(canonical_dest.as_path()) {
            return Ok(Self::to_relative(vault_path, &source));
        }
        let target = dest_dir.join(&name);
        if target.exists() {
            return Err(format!(
                "Ya existe un elemento llamado '{}' en el destino",
                name.to_string_lossy()
            ));
        }

        board_guard::ensure_writable(&source)?;
        board_guard::ensure_writable(&target)?;
        rename_with_board(&source, &target)
            .map_err(|e| format!("Error al mover elemento: {}", e))?;
        Ok(Self::to_relative(vault_path, &target))
    }

    /// Renombra un elemento sin moverlo fuera de su carpeta actual.
    pub fn rename_item(
        vault_path: &Path,
        source_rel: &str,
        new_name: &str,
    ) -> Result<String, String> {
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

        let parent = source
            .parent()
            .ok_or_else(|| "Elemento sin carpeta padre".to_string())?;
        let target = parent.join(new_name);
        if target == source {
            return Ok(Self::to_relative(vault_path, &source));
        }
        if target.exists() {
            return Err(format!("Ya existe un elemento llamado '{}'", new_name));
        }

        board_guard::ensure_writable(&source)?;
        board_guard::ensure_writable(&target)?;
        rename_with_board(&source, &target)
            .map_err(|e| format!("Error al renombrar elemento: {}", e))?;

        // Markdown renombrado: si su frontmatter tenía `title` igual al nombre anterior, pasa al nuevo
        // (un título propio, distinto del nombre, se respeta; sin `title` no se añade)
        if target.is_file() {
            let file_types = crate::domain::vault::file_types::SupportedFileTypes::default();
            let target_name = target.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_types.is_markdown_file(target_name) && !file_types.is_drawing_file(target_name)
                && let Ok(raw_content) = fs::read_to_string(&target) {
                    let old_stem = source.file_stem().and_then(|s| s.to_str());
                    if let (Some(old_stem), Some(new_stem)) = (old_stem, target.file_stem().and_then(|s| s.to_str()))
                        && let Some(updated) = crate::domain::note::frontmatter::NoteFrontmatter::rename_title(
                            &raw_content,
                            old_stem,
                            new_stem,
                        ) {
                            let _ = fs::write(&target, updated);
                        }
                }
        }

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

        board_guard::ensure_writable(&target_path)?;
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
    pub fn write_binary_item(
        vault_path: &Path,
        relative_path: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let target = Self::resolve(vault_path, relative_path)?;
        if target == vault_path {
            return Err("No se puede escribir directamente en la raíz de la bóveda".to_string());
        }
        board_guard::ensure_writable(&target)?;

        let canonical_vault = vault_path.canonicalize().map_err(|e| e.to_string())?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Error al crear directorio: {}", e))?;
            let canonical_parent = parent.canonicalize().map_err(|e| e.to_string())?;
            if !canonical_parent.starts_with(&canonical_vault) {
                return Err("Operación no permitida: fuera de los límites de la bóveda".to_string());
            }
        }

        fs::write(&target, bytes)
            .map_err(|e| format!("Error al escribir archivo binario: {}", e))?;
        Ok(())
    }

    /// Lee contenido binario de un archivo dentro de la bóveda garantizando los límites de seguridad.
    pub fn read_binary_item(vault_path: &Path, relative_path: &str) -> Result<Vec<u8>, String> {
        let target = Self::resolve(vault_path, relative_path)?;
        if !target.is_file() {
            return Err(format!(
                "El archivo no existe o no es un archivo: {}",
                relative_path
            ));
        }

        fs::read(&target).map_err(|e| format!("Error al leer archivo binario: {}", e))
    }

    /// Crea una carpeta dentro de la bóveda garantizando los límites de seguridad.
    pub fn create_folder(vault_path: &Path, relative_path: &str) -> Result<(), String> {
        let target = Self::resolve(vault_path, relative_path)?;
        board_guard::ensure_writable(&target)?;
        fs::create_dir_all(&target).map_err(|e| format!("Error al crear carpeta: {}", e))
    }
}

use crate::domain::vault::folder::FolderEntry;
use crate::domain::vault::vault_item_repository::{VaultFile, VaultItemRepository};

impl VaultItemRepository for VaultItemRepositoryImpl {
    fn delete_item(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        Self::delete_item(vault_path, relative_path)
    }

    fn rename_item(
        &self,
        vault_path: &Path,
        relative_path: &str,
        new_name: &str,
    ) -> Result<String, String> {
        Self::rename_item(vault_path, relative_path, new_name)
    }

    fn move_item(
        &self,
        vault_path: &Path,
        source_rel: &str,
        dest_dir_rel: &str,
    ) -> Result<String, String> {
        Self::move_item(vault_path, source_rel, dest_dir_rel)
    }

    fn copy_item(
        &self,
        vault_path: &Path,
        source_rel: &str,
        dest_dir_rel: &str,
    ) -> Result<String, String> {
        Self::copy_item(vault_path, source_rel, dest_dir_rel)
    }

    fn import_item(
        &self,
        vault_path: &Path,
        source_abs: &Path,
        dest_dir_rel: &str,
    ) -> Result<String, String> {
        Self::import_item(vault_path, source_abs, dest_dir_rel)
    }

    fn write_binary_item(
        &self,
        vault_path: &Path,
        relative_path: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        Self::write_binary_item(vault_path, relative_path, bytes)
    }

    fn read_binary_item(&self, vault_path: &Path, relative_path: &str) -> Result<Vec<u8>, String> {
        Self::read_binary_item(vault_path, relative_path)
    }

    fn create_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        Self::create_folder(vault_path, relative_path)
    }

    fn ensure_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        let target = vault_path.join(relative_path);
        board_guard::ensure_writable(&target)?;
        fs::create_dir_all(target).map_err(|e| e.to_string())
    }

    fn relocate(&self, vault_path: &Path, source_rel: &str, dest_rel: &str) -> Result<(), String> {
        let (source, target) = (vault_path.join(source_rel), vault_path.join(dest_rel));
        board_guard::ensure_writable(&source)?;
        board_guard::ensure_writable(&target)?;
        fs::rename(source, target).map_err(|e| e.to_string())
    }

    fn list_folder(
        &self,
        vault_path: &Path,
        relative_path: &str,
    ) -> Result<Vec<FolderEntry>, String> {
        let entries = fs::read_dir(vault_path.join(relative_path)).map_err(|e| e.to_string())?;
        Ok(entries
            .flatten()
            .map(|entry| FolderEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir: entry.file_type().map(|t| t.is_dir()).unwrap_or(false),
            })
            .collect())
    }

    fn exists(&self, vault_path: &Path, relative_path: &str) -> bool {
        vault_path.join(relative_path).exists()
    }

    fn is_folder(&self, vault_path: &Path, relative_path: &str) -> bool {
        let path = vault_path.join(relative_path);
        path.exists() && path.is_dir()
    }

    fn list_files_recursive(&self, vault_path: &Path, folder: &str) -> Result<Vec<String>, String> {
        let source_dir = vault_path.join(folder);
        let entries = jwalk::WalkDir::new(&source_dir)
            .skip_hidden(true)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut files = Vec::new();
        for entry in entries {
            if entry.file_type.is_file()
                && let Ok(rel) = entry.path().strip_prefix(&source_dir) {
                    files.push(rel.to_string_lossy().replace('\\', "/"));
                }
        }
        Ok(files)
    }

    fn list_all_folders(&self, vault_path: &Path) -> Result<Vec<String>, String> {
        let entries = jwalk::WalkDir::new(vault_path)
            .skip_hidden(true)
            .process_read_dir(|depth, _path, _state, children| {
                if depth.is_none() {
                    return;
                }
                children.retain(|entry_res| {
                    if let Ok(entry) = entry_res {
                        let name = entry.file_name.to_string_lossy();
                        !name.starts_with('.') && name != "target" && name != "node_modules"
                    } else {
                        true
                    }
                });
            })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut folders = Vec::new();
        for entry in entries {
            if entry.file_type.is_dir()
                && let Ok(rel) = entry.path().strip_prefix(vault_path) {
                    let clean = rel
                        .to_string_lossy()
                        .replace('\\', "/")
                        .trim_matches('/')
                        .to_string();
                    if !clean.is_empty() {
                        folders.push(clean);
                    }
                }
        }
        Ok(folders)
    }

    fn read_text(&self, vault_path: &Path, relative_path: &str) -> Result<String, String> {
        fs::read_to_string(vault_path.join(relative_path)).map_err(|e| e.to_string())
    }

    fn list_vault_files(&self, vault_path: &Path, folder: &str) -> Result<Vec<VaultFile>, String> {
        let folder = folder.trim_matches('/');
        let root = Self::resolve(vault_path, folder)?;
        if !root.is_dir() {
            return Err(format!("La carpeta '{folder}' no existe o no es un directorio"));
        }
        let mut files: Vec<VaultFile> = jwalk::WalkDir::new(&root)
            .skip_hidden(true)
            .process_read_dir(|depth, _path, _state, children| {
                if depth.is_none() {
                    return;
                }
                children.retain(|entry_res| match entry_res {
                    Ok(entry) => !crate::infrastructure::entrypoint::subscriber::vault_watcher::is_ignored_dir_or_file(
                        &entry.file_name.to_string_lossy(),
                    ),
                    Err(_) => true,
                });
            })
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type.is_file())
            .filter_map(|entry| {
                let path = entry.path();
                let rel = path.strip_prefix(vault_path).ok()?;
                Some(VaultFile {
                    relative_path: rel.to_string_lossy().replace('\\', "/"),
                    size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                })
            })
            .collect();
        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("synapse_fs_test_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("nota.md"), "hola").unwrap();
        dir
    }

    #[test]
    fn copy_same_folder_appends_copy() {
        let vault = temp_vault("same");
        let rel = VaultItemRepositoryImpl::copy_item(&vault, "nota.md", "").unwrap();
        assert_eq!(rel, "nota.copy.md");
        let rel2 = VaultItemRepositoryImpl::copy_item(&vault, "nota.md", "").unwrap();
        assert_eq!(rel2, "nota.copy.copy.md");
        let _ = fs::remove_dir_all(&vault);
    }

    #[test]
    fn copy_other_folder_keeps_name() {
        let vault = temp_vault("other");
        let rel = VaultItemRepositoryImpl::copy_item(&vault, "nota.md", "sub").unwrap();
        assert_eq!(rel, "sub/nota.md");
        let _ = fs::remove_dir_all(&vault);
    }

    #[test]
    fn import_external_file_into_folder() {
        let vault = temp_vault("import");
        let outside = temp_vault("import_src");
        fs::write(outside.join("foto.png"), "png").unwrap();
        let rel = VaultItemRepositoryImpl::import_item(&vault, &outside.join("foto.png"), "sub").unwrap();
        assert_eq!(rel, "sub/foto.png");
        // Si ya existe, no se pisa
        let rel2 = VaultItemRepositoryImpl::import_item(&vault, &outside.join("foto.png"), "sub").unwrap();
        assert_eq!(rel2, "sub/foto.copy.png");
        assert!(VaultItemRepositoryImpl::import_item(&vault, Path::new("relativa.md"), "").is_err());
        let _ = fs::remove_dir_all(&vault);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn copy_folder_into_itself_fails() {
        let vault = temp_vault("self");
        assert!(VaultItemRepositoryImpl::copy_item(&vault, "sub", "sub").is_err());
        let rel = VaultItemRepositoryImpl::copy_item(&vault, "sub", "").unwrap();
        assert_eq!(rel, "sub.copy");
        let _ = fs::remove_dir_all(&vault);
    }

    #[test]
    fn write_and_read_binary_item() {
        let vault = temp_vault("binary");
        let pdf_bytes = b"%PDF-1.7\nsome binary content\n%%EOF";
        let res = VaultItemRepositoryImpl::write_binary_item(&vault, "docs/sample.pdf", pdf_bytes);
        assert!(res.is_ok());

        let read_bytes = fs::read(vault.join("docs/sample.pdf")).unwrap();
        assert_eq!(read_bytes, pdf_bytes);

        // Test read_binary_item directly
        let direct_read =
            VaultItemRepositoryImpl::read_binary_item(&vault, "docs/sample.pdf").unwrap();
        assert_eq!(direct_read, pdf_bytes);

        // Test create_folder
        let folder_res =
            VaultItemRepositoryImpl::create_folder(&vault, "archivos/adjuntos/excalidraw");
        assert!(folder_res.is_ok());
        assert!(vault.join("archivos/adjuntos/excalidraw").is_dir());

        let _ = fs::remove_dir_all(&vault);
    }
}

#[cfg(test)]
mod move_tests {
    use super::VaultItemRepositoryImpl;
    use std::fs;

    #[test]
    fn moves_files_and_rejects_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();
        fs::create_dir_all(vault.join("sub/inner")).unwrap();
        fs::write(vault.join("nota.md"), "x").unwrap();
        assert_eq!(
            VaultItemRepositoryImpl::move_item(vault, "nota.md", "sub").unwrap(),
            "sub/nota.md"
        );
        assert!(!vault.join("nota.md").exists());
        // mismo lugar: no-op
        assert_eq!(
            VaultItemRepositoryImpl::move_item(vault, "sub/nota.md", "sub").unwrap(),
            "sub/nota.md"
        );
        // conflicto de nombre: no sobrescribe
        fs::write(vault.join("nota.md"), "y").unwrap();
        assert!(VaultItemRepositoryImpl::move_item(vault, "nota.md", "sub").is_err());
        assert_eq!(fs::read_to_string(vault.join("sub/nota.md")).unwrap(), "x");
        // carpeta dentro de sí misma
        assert!(VaultItemRepositoryImpl::move_item(vault, "sub", "sub/inner").is_err());
        // carpeta a la raíz
        assert_eq!(
            VaultItemRepositoryImpl::move_item(vault, "sub/inner", "").unwrap(),
            "inner"
        );
    }
}
