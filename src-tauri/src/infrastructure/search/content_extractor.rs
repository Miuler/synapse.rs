use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::full_text::DocumentKind;
use crate::infrastructure::repositories::file_note_repository::detect_and_decode;
use crate::infrastructure::navigation::engine::extract_title_from_markdown;
use pulldown_cmark::{Event, Options, Parser, TagEnd};
use std::path::Path;

const MAX_INDEXABLE_SIZE: usize = 5 * 1024 * 1024; // 5 MB

/// Extrae texto limpio y metadatos para la indexación full-text.
pub fn extract(
    rel_path: &Path,
    bytes: &[u8],
    file_types: &SupportedFileTypes,
) -> Option<(DocumentKind, String, String)> {
    // 1. Descartar archivos mayores al límite
    if bytes.len() > MAX_INDEXABLE_SIZE {
        return None;
    }

    // 2. Descartar archivos binarios (detectando byte nulo en los primeros bytes)
    let check_slice = &bytes[..bytes.len().min(8000)];
    if check_slice.contains(&0) {
        return None;
    }

    let file_name = rel_path.file_name()?.to_str()?;
    let lower_name = file_name.to_lowercase();
    let file_stem = rel_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);

    // 3. Excalidraw: `.excalidraw` o `.excalidraw.json`
    // Ignoramos formatos de exportación de imagen: .excalidraw.svg, .excalidraw.png
    if lower_name.ends_with(".excalidraw")
        || lower_name.ends_with(".excalidraw.json")
    {
        let (_, decoded) = detect_and_decode(bytes);
        let extracted_text = extract_excalidraw_text(&decoded);
        return Some((
            DocumentKind::Excalidraw,
            file_stem.to_string(),
            extracted_text,
        ));
    }

    // Si es .excalidraw.svg o .excalidraw.png, no se indexa como documento de dibujo
    if lower_name.ends_with(".excalidraw.svg") || lower_name.ends_with(".excalidraw.png") {
        return None;
    }

    // 4. Markdown
    let is_markdown = file_types
        .markdown
        .iter()
        .any(|ext| lower_name.ends_with(&format!(".{}", ext.to_lowercase())));

    if is_markdown {
        let (_, decoded) = detect_and_decode(bytes);
        let title = extract_title_from_markdown(&decoded, file_stem);
        let plain_body = extract_markdown_plain_text(&decoded);
        return Some((DocumentKind::Markdown, title, plain_body));
    }

    // 5. Diagramas (Mermaid)
    let is_diagram = file_types
        .diagrams
        .iter()
        .any(|ext| lower_name.ends_with(&format!(".{}", ext.to_lowercase())));

    if is_diagram {
        let (_, decoded) = detect_and_decode(bytes);
        return Some((DocumentKind::Mermaid, file_name.to_string(), decoded));
    }

    // 6. Código y texto estructurado
    let is_code = file_types
        .code
        .iter()
        .any(|ext| lower_name.ends_with(&format!(".{}", ext.to_lowercase())));

    if is_code {
        let (_, decoded) = detect_and_decode(bytes);
        return Some((DocumentKind::Code, file_name.to_string(), decoded));
    }

    None
}

/// Extrae únicamente los textos de los elementos activos en un diagrama Excalidraw.
fn extract_excalidraw_text(json_str: &str) -> String {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return String::new();
    };

    let mut texts = Vec::new();

    if let Some(elements) = val.get("elements").and_then(|e| e.as_array()) {
        for elem in elements {
            if elem.get("isDeleted").and_then(|d| d.as_bool()) == Some(true) {
                continue;
            }

            if let Some(text) = elem.get("text").and_then(|t| t.as_str()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    texts.push(trimmed.to_string());
                }
            } else if let Some(orig) = elem.get("originalText").and_then(|t| t.as_str()) {
                let trimmed = orig.trim();
                if !trimmed.is_empty() {
                    texts.push(trimmed.to_string());
                }
            }
        }
    }

    texts.join("\n")
}

/// Convierte contenido Markdown a texto plano limpio sin marcado de sintaxis.
pub fn extract_markdown_plain_text(content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let parser = Parser::new_ext(content, options);
    let mut plain = String::with_capacity(content.len());

    for event in parser {
        match event {
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) | Event::DisplayMath(t) => {
                plain.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => {
                plain.push(' ');
            }
            Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Heading(_))
            | Event::End(TagEnd::Item)
            | Event::End(TagEnd::TableCell)
            | Event::End(TagEnd::CodeBlock)
            | Event::End(TagEnd::MetadataBlock(_)) => {
                plain.push('\n');
            }
            _ => {}
        }
    }

    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_markdown() {
        let md = "# Mi Título\n\nEste es un párrafo con **negrita** y [enlace](https://example.com).\n\n```rust\nlet x = 42;\n```";
        let path = Path::new("test.md");
        let file_types = SupportedFileTypes::default();
        let res = extract(path, md.as_bytes(), &file_types);
        assert!(res.is_some());
        let (kind, title, body) = res.unwrap();
        assert_eq!(kind, DocumentKind::Markdown);
        assert_eq!(title, "Mi Título");
        assert!(body.contains("Este es un párrafo con negrita y enlace."));
        assert!(body.contains("let x = 42;"));
    }

    #[test]
    fn test_extract_excalidraw() {
        let json = r#"{
            "type": "excalidraw",
            "elements": [
                { "type": "text", "text": "Nodo A", "isDeleted": false },
                { "type": "text", "text": "Nodo Borrado", "isDeleted": true },
                { "type": "rectangle", "isDeleted": false }
            ]
        }"#;
        let path = Path::new("diagram.excalidraw");
        let file_types = SupportedFileTypes::default();
        let res = extract(path, json.as_bytes(), &file_types);
        assert!(res.is_some());
        let (kind, title, body) = res.unwrap();
        assert_eq!(kind, DocumentKind::Excalidraw);
        assert_eq!(title, "diagram");
        assert!(body.contains("Nodo A"));
        assert!(!body.contains("Nodo Borrado"));
    }

    #[test]
    fn test_ignore_binary_and_huge() {
        let file_types = SupportedFileTypes::default();
        let path = Path::new("blob.md");
        let mut binary = vec![b'a'; 100];
        binary[50] = 0;
        assert!(extract(path, &binary, &file_types).is_none());
    }

    #[test]
    fn test_extract_cobol_file() {
        let cobol_src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO-WORLD.\n       PROCEDURE DIVISION.\n           DISPLAY 'HELLO COBOL'.\n           STOP RUN.";
        let file_types = SupportedFileTypes::default();
        let path = Path::new("subfolder/hello.cbl");
        let res = extract(path, cobol_src.as_bytes(), &file_types);
        assert!(res.is_some());
        let (kind, title, body) = res.unwrap();
        assert_eq!(kind, DocumentKind::Code);
        assert_eq!(title, "hello.cbl");
        assert!(body.contains("HELLO COBOL"));
    }
}
