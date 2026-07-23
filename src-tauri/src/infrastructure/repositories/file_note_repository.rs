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

    /// Función auxiliar recursiva para listar todos los archivos que coincidan con las extensiones en la bóveda
    fn walk_directory(
        base_vault: &Path,
        current_dir: &Path,
        notes: &mut Vec<Note>,
        extensions: &[String],
    ) -> Result<(), String> {
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
                Self::walk_directory(base_vault, &path, notes, extensions)?;
            } else if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let is_supported = extensions.iter().any(|e| e == ext);
                if is_supported {
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
    fn list_notes(&self, vault_path: &Path, extensions: &[String]) -> Result<Vec<Note>, String> {
        if !vault_path.exists() {
            fs::create_dir_all(vault_path).map_err(|e| e.to_string())?;
        }

        let mut notes = Vec::new();
        Self::walk_directory(vault_path, vault_path, &mut notes, extensions)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Once;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing_indicatif::IndicatifLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    static INIT: Once = Once::new();

    /// Setup function to initialize tracing once for all tests (simulates before_all)
    fn setup_tracing() {
        INIT.call_once(|| {
            // Establece el nivel INFO por defecto (o respeta la variable de entorno RUST_LOG si existe)
            let filter = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("debug"));
            let fmt_layer = tracing_subscriber::fmt::layer().with_test_writer();

            let indicatif_layer = IndicatifLayer::new();
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(indicatif_layer)
                .with(fmt_layer)
                .try_init();
        });
    }


    #[test]
    fn test_walk_directory() {
        setup_tracing();

        // Crear un directorio temporal único para la prueba
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let temp_dir = std::env::temp_dir().join(format!("synapse_test_{}", nanos));
        info!("temp_dir: {:?}", temp_dir);

        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Estructura de prueba:
        // temp_dir/
        //   ├── note1.md
        //   ├── subfolder/
        //   │     └── note2.markdown
        //   ├── ignored.txt
        //   ├── .hidden_dir/
        //   │     └── note_hidden.md
        //   └── target/
        //         └── note_target.md

        let bin1_path = temp_dir.join("other.bin");
        let mut bin1 = File::create(&bin1_path).unwrap();
        writeln!(bin1, "# bin 1").unwrap();

        let file1_path = temp_dir.join("file1.rs");
        let mut rs1 = File::create(&file1_path).unwrap();
        writeln!(rs1, "# Contenido.rs 1").unwrap();

        let note1_path = temp_dir.join("note1.md");
        let mut f1 = File::create(&note1_path).unwrap();
        writeln!(f1, "# Contenido 1").unwrap();

        let sub_dir = temp_dir.join("subfolder");
        fs::create_dir_all(&sub_dir).unwrap();
        let note2_path = sub_dir.join("note2.markdown");
        let mut f2 = File::create(&note2_path).unwrap();
        writeln!(f2, "# Contenido 2").unwrap();

        let ignored_path = temp_dir.join("ignored.txt");
        let mut f3 = File::create(&ignored_path).unwrap();
        writeln!(f3, "texto plano").unwrap();

        let hidden_dir = temp_dir.join(".hidden_dir");
        fs::create_dir_all(&hidden_dir).unwrap();
        let hidden_note = hidden_dir.join("note_hidden.md");
        let mut f4 = File::create(&hidden_note).unwrap();
        writeln!(f4, "# Oculto").unwrap();

        let target_dir = temp_dir.join("target");
        fs::create_dir_all(&target_dir).unwrap();
        let target_note = target_dir.join("note_target.md");
        let mut f5 = File::create(&target_note).unwrap();
        writeln!(f5, "# Target").unwrap();

        // Ejecutar walk_directory
        let mut notes = Vec::new();
        let extensions = vec!["md".to_string(), "markdown".to_string(), "rs".to_string()];
        let result = FileNoteRepository::walk_directory(&temp_dir, &temp_dir, &mut notes, &extensions);

        // Limpieza
        // let _ = fs::remove_dir_all(&temp_dir);

        assert!(result.is_ok());
        // Debería encontrar exactamente note1.md y subfolder/note2.markdown
        assert_eq!(notes.len(), 3);

        let titles: Vec<String> = notes.iter().map(|n| n.title.clone()).collect();
        assert!(titles.contains(&"note1".to_string()));
        assert!(titles.contains(&"note2".to_string()));
    }

    #[test]
    fn test_walk_directory_rs() {
        setup_tracing();

        let mut notes = Vec::new();
        let extensions = vec!["md".to_string(), "markdown".to_string(), "rs".to_string()];
        let project_path = PathBuf::from("/home/miuler/src/github.com/Miuler/synapse.rs");
        FileNoteRepository::walk_directory(&project_path, &project_path, &mut notes, &extensions).unwrap();
        info!("len: {}", notes.len())
    }
}