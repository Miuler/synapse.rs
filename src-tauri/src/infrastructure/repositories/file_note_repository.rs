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
}

impl NoteRepository for FileNoteRepository {
    fn list_notes(&self, vault_path: &Path) -> Result<Vec<Note>, String> {
        if !vault_path.exists() {
            return Ok(Vec::new());
        }

        let mut notes = Vec::new();
        let entries = fs::read_dir(vault_path).map_err(|e| e.to_string())?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if let Ok(rel_path) = NoteRelativePath::new(file_name) {
                        let content = fs::read_to_string(&path).unwrap_or_default();
                        let title = file_name.trim_end_matches(".md").to_string();
                        notes.push(Note::new(rel_path, title, content));
                    }
                }
            }
        }

        Ok(notes)
    }

    fn read_note(&self, vault_path: &Path, relative_path: &NoteRelativePath) -> Result<Note, String> {
        let abs_path = self.resolve_absolute_path(vault_path, relative_path);
        let content = fs::read_to_string(&abs_path).map_err(|e| format!("Error al leer la nota en {:?}: {}", abs_path, e))?;
        
        let title = abs_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Sin título")
            .trim_end_matches(".md")
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
