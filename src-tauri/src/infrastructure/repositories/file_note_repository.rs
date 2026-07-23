use crate::domain::models::note::Note;
use crate::domain::repositories::note_repository::NoteRepository;
use crate::domain::value_objects::note_path::NoteRelativePath;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileNoteRepository;

impl FileNoteRepository {
    pub fn new() -> Self {
        Self
    }

    fn resolve_absolute_path(&self, vault_path: &Path, relative_path: &NoteRelativePath) -> PathBuf {
        vault_path.join(relative_path.as_str())
    }

    /// Función auxiliar recursiva para listar todos los archivos Markdown en la bóveda
    fn walk_directory(base_vault: &Path, current_dir: &Path, notes: &mut Vec<Note>) -> Result<(), String> {
        if !current_dir.exists() || !current_dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(current_dir).map_err(|e| e.to_string())?;

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };

            // Ignorar directorios ocultos y carpetas de build/dependencias
            if file_name.starts_with('.') || file_name == "node_modules" || file_name == "target" {
                continue;
            }

            if path.is_dir() {
                Self::walk_directory(base_vault, &path, notes)?;
            } else if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let is_markdown = ext == "md" || ext == "markdown";
                if is_markdown {
                    if let Ok(rel_path_buf) = path.strip_prefix(base_vault) {
                        if let Some(rel_str) = rel_path_buf.to_str() {
                            if let Ok(rel_path) = NoteRelativePath::new(rel_str) {
                                let content = fs::read_to_string(&path).unwrap_or_default();
                                let title = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(file_name)
                                    .to_string();
                                notes.push(Note::new(rel_path, title, content));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl NoteRepository for FileNoteRepository {
    fn list_notes(&self, vault_path: &Path) -> Result<Vec<Note>, String> {
        if !vault_path.exists() {
            fs::create_dir_all(vault_path).map_err(|e| e.to_string())?;
        }

        let mut notes = Vec::new();
        Self::walk_directory(vault_path, vault_path, &mut notes)?;

        // Ordenar notas alfabéticamente por título
        notes.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        Ok(notes)
    }

    fn read_note(&self, vault_path: &Path, relative_path: &NoteRelativePath) -> Result<Note, String> {
        let abs_path = self.resolve_absolute_path(vault_path, relative_path);
        let content = fs::read_to_string(&abs_path).map_err(|e| format!("Error al leer la nota en {:?}: {}", abs_path, e))?;
        
        let title = abs_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Sin título")
            .to_string();

        Ok(Note::new(relative_path.clone(), title, content))
    }

    fn save_note(&self, vault_path: &Path, note: &Note) -> Result<(), String> {
        let abs_path = self.resolve_absolute_path(vault_path, &note.relative_path);
        
        // Crear directorio padre si no existe
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        fs::write(&abs_path, &note.content).map_err(|e| format!("Error al guardar la nota en {:?}: {}", abs_path, e))
    }
}
