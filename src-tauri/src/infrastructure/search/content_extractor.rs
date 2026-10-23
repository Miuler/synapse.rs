use crate::domain::models::config::IndexingConfig;
use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::full_text::DocumentKind;
use crate::infrastructure::navigation::engine::extract_title_from_markdown;
use crate::infrastructure::repositories::file_note_repository::detect_and_decode;
use pulldown_cmark::{Event, Options, Parser, TagEnd};
use std::path::Path;

const MAX_INDEXABLE_SIZE: usize = 5 * 1024 * 1024; // 5 MB

fn truncate_safe(s: &mut String, max_chars: usize) {
    if s.len() > max_chars {
        let mut end = max_chars;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        s.truncate(end);
    }
}

/// Extrae texto limpio y metadatos para la indexación full-text usando configuración por defecto.
pub fn extract(
    rel_path: &Path,
    bytes: &[u8],
    file_types: &SupportedFileTypes,
) -> Option<(DocumentKind, String, String)> {
    let default_config = IndexingConfig::default();
    extract_with_config(rel_path, bytes, file_types, &default_config)
}

/// Extrae texto limpio y metadatos para la indexación full-text aplicando los límites configurados.
pub fn extract_with_config(
    rel_path: &Path,
    bytes: &[u8],
    file_types: &SupportedFileTypes,
    indexing: &IndexingConfig,
) -> Option<(DocumentKind, String, String)> {
    let file_name = rel_path.file_name()?.to_str()?;
    let lower_name = file_name.to_lowercase();
    let file_stem = rel_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);

    // 1. Documentos PDF
    if file_types.is_pdf_file(&lower_name) || lower_name.ends_with(".pdf") {
        if bytes.len() > indexing.max_pdf_file_size {
            return None;
        }
        let text = super::pdf_extractor::extract_pdf_text(
            bytes,
            indexing.max_pdf_pages,
            indexing.max_extracted_chars,
            indexing.max_word_length,
        )
        .ok()?;
        if text.trim().is_empty() {
            return None;
        }
        return Some((DocumentKind::Pdf, file_stem.to_string(), text));
    }

    // 2. Documentos Microsoft Word (.docx)
    if lower_name.ends_with(".docx") {
        if bytes.len() > indexing.max_docx_file_size {
            return None;
        }
        let text = super::docx_extractor::extract_docx_text(
            bytes,
            indexing.max_extracted_chars,
            indexing.max_word_length,
        )
        .ok()?;
        if text.trim().is_empty() {
            return None;
        }
        return Some((DocumentKind::Word, file_stem.to_string(), text));
    }

    // 3. Descartar archivos mayores al límite para texto plano
    if bytes.len() > MAX_INDEXABLE_SIZE {
        return None;
    }

    // 4. Descartar otros archivos binarios (detectando byte nulo en los primeros bytes)
    let check_slice = &bytes[..bytes.len().min(8000)];
    if check_slice.contains(&0) {
        return None;
    }

    // 5. Excalidraw: `.excalidraw`, `.excalidraw.json`, `.excalidraw.md` o `.excalidraw.excalidraw`
    // Ignoramos formatos de exportación de imagen: .excalidraw.svg, .excalidraw.png
    if lower_name.ends_with(".excalidraw")
        || lower_name.ends_with(".excalidraw.json")
        || lower_name.ends_with(".excalidraw.md")
        || lower_name.ends_with(".excalidraw.excalidraw")
    {
        let (_, decoded) = detect_and_decode(bytes);
        let mut extracted_text = extract_excalidraw_text(&decoded);
        truncate_safe(&mut extracted_text, indexing.max_extracted_chars);
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

    // 6. Markdown
    let is_markdown = file_types
        .markdown
        .iter()
        .any(|ext| lower_name.ends_with(&format!(".{}", ext.to_lowercase())));

    if is_markdown {
        let (_, decoded) = detect_and_decode(bytes);

        // Detección por Frontmatter YAML o marcadores de Obsidian Excalidraw
        if decoded.contains("excalidraw-plugin:") || decoded.contains("```compressed-json") {
            let mut extracted_text = extract_excalidraw_text(&decoded);
            truncate_safe(&mut extracted_text, indexing.max_extracted_chars);
            return Some((
                DocumentKind::Excalidraw,
                file_stem.to_string(),
                extracted_text,
            ));
        }

        let title = extract_title_from_markdown(&decoded, file_stem);
        let mut plain_body = extract_markdown_plain_text(&decoded);
        truncate_safe(&mut plain_body, indexing.max_extracted_chars);
        return Some((DocumentKind::Markdown, title, plain_body));
    }

    // 7. Diagramas (Mermaid)
    let is_diagram = file_types
        .diagrams
        .iter()
        .any(|ext| lower_name.ends_with(&format!(".{}", ext.to_lowercase())));

    if is_diagram {
        let (_, mut decoded) = detect_and_decode(bytes);
        truncate_safe(&mut decoded, indexing.max_extracted_chars);
        return Some((DocumentKind::Mermaid, file_name.to_string(), decoded));
    }

    // 8. Código y texto estructurado
    let is_code = file_types
        .code
        .iter()
        .any(|ext| lower_name.ends_with(&format!(".{}", ext.to_lowercase())));

    if is_code {
        let (_, mut decoded) = detect_and_decode(bytes);
        truncate_safe(&mut decoded, indexing.max_extracted_chars);
        return Some((DocumentKind::Code, file_name.to_string(), decoded));
    }

    None
}

/// Extrae únicamente los textos de los elementos activos en un diagrama Excalidraw.
fn extract_excalidraw_text(raw_str: &str) -> String {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw_str) {
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

        return texts.join("\n");
    }

    // Si es formato Markdown de Excalidraw (Obsidian), extraer líneas de la sección '# Text Elements'
    if let Some(pos) = raw_str.find("# Text Elements") {
        let text_part = &raw_str[pos + 15..];
        let end_pos = text_part.find("%%").unwrap_or(text_part.len());
        let section = &text_part[..end_pos];
        let mut texts = Vec::new();
        for line in section.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // Eliminar identificador de bloque de Obsidian tipo ^abc123
            let clean = if let Some(caret_idx) = trimmed.rfind('^') {
                trimmed[..caret_idx].trim()
            } else {
                trimmed
            };
            if !clean.is_empty() {
                texts.push(clean.to_string());
            }
        }
        if !texts.is_empty() {
            return texts.join("\n");
        }
    }

    String::new()
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
    fn test_extract_excalidraw_markdown() {
        let md = r#"---

excalidraw-plugin: parsed
tags: [excalidraw]

---
==Deconstruct of drawing==
# Text Elements
Elemento 1 ^abc123

Elemento 2 ^def456

%%
# Drawing
```json
{}
```
%%
"#;
        let path = Path::new("drawing.excalidraw.md");
        let file_types = SupportedFileTypes::default();
        let res = extract(path, md.as_bytes(), &file_types);
        assert!(res.is_some());
        let (kind, title, body) = res.unwrap();
        assert_eq!(kind, DocumentKind::Excalidraw);
        assert_eq!(title, "drawing.excalidraw");
        assert!(body.contains("Elemento 1"));
        assert!(body.contains("Elemento 2"));
        assert!(!body.contains("^abc123"));
    }

    #[test]
    fn test_extract_excalidraw_from_markdown_frontmatter() {
        let md = r#"---
excalidraw-plugin: parsed
tags: [arquitectura]
---
# Text Elements
Microservicio Pagos ^p1

Base de Datos PostgreSQL ^db1
"#;
        let path = Path::new("arquitectura_sistema.md");
        let file_types = SupportedFileTypes::default();
        let res = extract(path, md.as_bytes(), &file_types);
        assert!(res.is_some());
        let (kind, title, body) = res.unwrap();
        assert_eq!(kind, DocumentKind::Excalidraw);
        assert_eq!(title, "arquitectura_sistema");
        assert!(body.contains("Microservicio Pagos"));
        assert!(body.contains("Base de Datos PostgreSQL"));
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

    #[test]
    fn test_extract_docx_file() {
        use std::io::{Cursor, Write};
        use zip::write::{SimpleFileOptions, ZipWriter};

        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default();
            zip.start_file("word/document.xml", options).unwrap();
            let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:body><w:p><w:r><w:t>Contenido extraído del archivo Word</w:t></w:r></w:p></w:body>
            </w:document>"#;
            zip.write_all(xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        let file_types = SupportedFileTypes::default();
        let path = Path::new("informes/reporte.docx");
        let res = extract(path, &buffer, &file_types);
        assert!(res.is_some());
        let (kind, title, body) = res.unwrap();
        assert_eq!(kind, DocumentKind::Word);
        assert_eq!(title, "reporte");
        assert!(body.contains("Contenido extraído del archivo Word"));
    }

    #[test]
    fn test_extract_pdf_file() {
        use lopdf::content::Content;
        use lopdf::{dictionary, Document, Object, Stream};

        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });

        let content = Content {
            operations: vec![
                lopdf::content::Operation::new("BT", vec![]),
                lopdf::content::Operation::new("Tf", vec!["F1".into(), 12.into()]),
                lopdf::content::Operation::new("Tj", vec![Object::string_literal("Texto indexable de PDF")]),
                lopdf::content::Operation::new("ET", vec![]),
            ],
        };

        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
        });

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();

        let file_types = SupportedFileTypes::default();
        let path = Path::new("libros/manual.pdf");
        let res = extract(path, &buffer, &file_types);
        assert!(res.is_some());
        let (kind, title, body) = res.unwrap();
        assert_eq!(kind, DocumentKind::Pdf);
        assert_eq!(title, "manual");
        assert!(body.contains("Texto indexable de PDF"));
    }
}
