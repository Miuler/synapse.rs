use crate::domain::models::note::Note;
use crate::domain::repositories::note_repository::NoteRepository;
use crate::domain::value_objects::note_path::NoteRelativePath;
use std::fs;
use std::path::{Path, PathBuf};

pub fn detect_and_decode(bytes: &[u8]) -> (String, String) {
    if bytes.is_empty() {
        return ("---".to_string(), String::new());
    }

    // 1. BOM UTF-8
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let content = String::from_utf8_lossy(&bytes[3..]).to_string();
        return ("UTF-8 con BOM".to_string(), content);
    }

    // 2. BOM UTF-16 LE
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (cow, _, _) = encoding_rs::UTF_16LE.decode(&bytes[2..]);
        return ("UTF-16 LE".to_string(), cow.into_owned());
    }

    // 3. BOM UTF-16 BE
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _, _) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
        return ("UTF-16 BE".to_string(), cow.into_owned());
    }

    // 4. ASCII puro (todos los bytes <= 127)
    let is_pure_ascii = bytes.iter().all(|&b| b <= 0x7F);
    if is_pure_ascii {
        let content = match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(bytes).to_string(),
        };
        return ("ASCII".to_string(), content);
    }

    // 5. UTF-8 estándar válido
    if let Ok(s) = std::str::from_utf8(bytes) {
        return ("UTF-8".to_string(), s.to_string());
    }

    // 6. Detección heurística para encodings heredados (Windows-1252, ISO-8859-1, etc.)
    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
    detector.feed(bytes, true);
    let guessed = detector.guess(None, chardetng::Utf8Detection::Deny);
    let (cow, _, _) = guessed.decode(bytes);

    let name = match guessed.name() {
        "windows-1252" => "Windows-1252",
        "windows-1250" => "Windows-1250",
        "windows-1251" => "Windows-1251",
        "windows-1256" => "Windows-1256",
        "ISO-8859-1" => "ISO-8859-1",
        "ISO-8859-2" => "ISO-8859-2",
        "ISO-8859-15" => "ISO-8859-15",
        "Shift_JIS" => "Shift_JIS",
        "GBK" => "GBK",
        "gb18030" => "GB18030",
        "Big5" => "Big5",
        "EUC-KR" => "EUC-KR",
        "EUC-JP" => "EUC-JP",
        other => other,
    };

    (name.to_string(), cow.into_owned())
}

pub fn encode_content(content: &str, encoding: &str) -> Vec<u8> {
    let enc_norm = encoding.trim().to_uppercase();
    match enc_norm.as_str() {
        "UTF-8 CON BOM" | "UTF-8 WITH BOM" => {
            let mut out = vec![0xEF, 0xBB, 0xBF];
            out.extend_from_slice(content.as_bytes());
            out
        }
        "UTF-16 LE" | "UTF-16LE" => {
            let mut out = vec![0xFF, 0xFE];
            for u in content.encode_utf16() {
                out.extend_from_slice(&u.to_le_bytes());
            }
            out
        }
        "UTF-16 BE" | "UTF-16BE" => {
            let mut out = vec![0xFE, 0xFF];
            for u in content.encode_utf16() {
                out.extend_from_slice(&u.to_be_bytes());
            }
            out
        }
        "ASCII" => {
            content
                .chars()
                .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
                .collect()
        }
        "ISO-8859-1" | "LATIN1" | "LATIN-1" => {
            content
                .chars()
                .map(|c| {
                    let cp = c as u32;
                    if cp <= 0xFF {
                        cp as u8
                    } else {
                        b'?'
                    }
                })
                .collect()
        }
        "---" | "" | "UTF-8" => content.as_bytes().to_vec(),
        _ => {
            let label = encoding.trim().to_lowercase();
            if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes()) {
                let (cow, _, _) = enc.encode(content);
                cow.into_owned()
            } else {
                content.as_bytes().to_vec()
            }
        }
    }
}

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
                                let content = String::new();
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
        let bytes = fs::read(&abs_path).map_err(|e| format!("Error al leer la nota en {:?}: {}", abs_path, e))?;
        let (encoding, content) = detect_and_decode(&bytes);
        
        let title = abs_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Sin título")
            .to_string();

        Ok(Note::with_encoding(relative_path.clone(), title, content, encoding))
    }

    fn save_note(&self, vault_path: &Path, note: &Note) -> Result<(), String> {
        let abs_path = self.resolve_absolute_path(vault_path, &note.relative_path);
        
        // Crear directorio padre si no existe
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let bytes = encode_content(&note.content, &note.encoding);
        fs::write(&abs_path, bytes).map_err(|e| format!("Error al guardar la nota en {:?}: {}", abs_path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;
    use std::fs::{self, File};
    use std::io::Write;
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
    fn test_encoding_detection_and_encode() {
        // Empty bytes
        let (enc_empty, text_empty) = detect_and_decode(b"");
        assert_eq!(enc_empty, "---");
        assert_eq!(text_empty, "");

        // UTF-8
        let utf8_bytes = "Hola, mundo! ñoño".as_bytes();
        let (enc, text) = detect_and_decode(utf8_bytes);
        assert_eq!(enc, "UTF-8");
        assert_eq!(text, "Hola, mundo! ñoño");

        // ASCII
        let ascii_bytes = b"Hello world 123";
        let (enc, text) = detect_and_decode(ascii_bytes);
        assert_eq!(enc, "ASCII");
        assert_eq!(text, "Hello world 123");

        // UTF-8 con BOM
        let mut bom_utf8 = vec![0xEF, 0xBB, 0xBF];
        bom_utf8.extend_from_slice(b"BOM text");
        let (enc, text) = detect_and_decode(&bom_utf8);
        assert_eq!(enc, "UTF-8 con BOM");
        assert_eq!(text, "BOM text");

        // Encode to UTF-16 LE and decode back
        let encoded_u16 = encode_content("Prueba UTF16", "UTF-16 LE");
        let (enc, text) = detect_and_decode(&encoded_u16);
        assert_eq!(enc, "UTF-16 LE");
        assert_eq!(text, "Prueba UTF16");
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

        assert!(result.is_ok());
        assert_eq!(notes.len(), 3);

        let titles: Vec<String> = notes.iter().map(|n| n.title.clone()).collect();
        assert!(titles.contains(&"note1".to_string()));
        assert!(titles.contains(&"note2".to_string()));
    }
}
