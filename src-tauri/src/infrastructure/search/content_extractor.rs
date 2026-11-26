use crate::domain::config::config::IndexingConfig;
use crate::domain::note::note::Note;
use crate::domain::search::full_text::DocumentKind;
use crate::domain::vault::file_types::SupportedFileTypes;
use crate::infrastructure::repository::file::note_repository_impl::detect_and_decode;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::path::Path;

const MAX_INDEXABLE_SIZE: usize = 5 * 1024 * 1024; // 5 MB
/// Texto máximo indexado de un modelo ArchiMate (ya sin el XML de los diagramas).
const ARCHIMATE_MAX_CHARS: usize = 2_000_000;

pub(super) fn truncate_safe(s: &mut String, max_chars: usize) {
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
) -> Option<(DocumentKind, String, String, String)> {
    let default_config = IndexingConfig::default();
    extract_with_config(rel_path, bytes, file_types, &default_config)
}

/// Extrae texto limpio y metadatos para la indexación full-text aplicando los límites configurados.
pub fn extract_with_config(
    rel_path: &Path,
    bytes: &[u8],
    file_types: &SupportedFileTypes,
    indexing: &IndexingConfig,
) -> Option<(DocumentKind, String, String, String)> {
    extract_document(rel_path, bytes, file_types, indexing)
        .map(|e| (e.kind, e.title, e.body, e.headings))
}

/// Lo que se indexa de un archivo.
#[derive(Debug, Clone, PartialEq)]
pub struct Extracted {
    pub kind: DocumentKind,
    pub title: String,
    pub body: String,
    pub headings: String,
    /// Posición de cada línea de `body` en el original (ver `FullTextDocument::layout`).
    pub layout: String,
    /// El texto se cortó por el límite de caracteres o de páginas.
    pub truncated: bool,
}

impl Extracted {
    fn plain(kind: DocumentKind, title: String, body: String, headings: String) -> Self {
        Self {
            kind,
            title,
            body,
            headings,
            layout: String::new(),
            truncated: false,
        }
    }
}

/// Como [`extract_with_config`], con la posición de cada línea y si el texto se cortó.
pub fn extract_document(
    rel_path: &Path,
    bytes: &[u8],
    file_types: &SupportedFileTypes,
    indexing: &IndexingConfig,
) -> Option<Extracted> {
    let mut e = extract_document_inner(rel_path, bytes, file_types, indexing)?;
    // Los formatos que no lo dicen ellos: el texto llegó al tope (`truncate_safe` corta en él)
    let cap = match e.kind {
        DocumentKind::Archimate => indexing.max_extracted_chars.max(ARCHIMATE_MAX_CHARS),
        _ => indexing.max_extracted_chars,
    };
    e.truncated |= e.body.len() + 4 >= cap;
    Some(e)
}

fn extract_document_inner(
    rel_path: &Path,
    bytes: &[u8],
    file_types: &SupportedFileTypes,
    indexing: &IndexingConfig,
) -> Option<Extracted> {
    let file_name = rel_path.file_name()?.to_str()?;
    let lower_name = file_name.to_lowercase();
    // El `index.md` OKF que genera Synapse no se indexa: repite los títulos de la carpeta
    if lower_name == crate::domain::okf::INDEX_FILE
        && crate::domain::okf::is_generated(&String::from_utf8_lossy(
            &bytes[..bytes.len().min(2048)],
        ))
    {
        return None;
    }
    let file_stem = rel_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);

    // 0. Diagramas de draw.io: `.drawio` (XML) o una imagen PNG/SVG con el diagrama dentro; se
    // indexan los textos de sus figuras. Una imagen sin diagrama no se indexa.
    let drawio_xml = if lower_name.ends_with(".drawio") {
        Some(String::from_utf8_lossy(bytes).into_owned())
    } else if lower_name.ends_with(".png") {
        super::drawio_extractor::png_mxfile(bytes)
    } else if lower_name.ends_with(".svg") {
        super::drawio_extractor::svg_mxfile(bytes)
    } else {
        None
    };
    if let Some(xml) = drawio_xml {
        let lines = super::drawio_extractor::mxfile_lines(&xml);
        let mut text = lines
            .iter()
            .map(|(t, _)| t.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        // `page` en las líneas que son nombres de página, para saber en qué página está cada texto
        let layout = lines
            .iter()
            .map(|(_, page)| if *page { "page" } else { "" })
            .collect::<Vec<_>>()
            .join("\n");
        let truncated = text.len() > indexing.max_extracted_chars;
        truncate_safe(&mut text, indexing.max_extracted_chars);
        let title = file_stem.trim_end_matches(".drawio").to_string();
        return Some(Extracted {
            layout,
            truncated,
            ..Extracted::plain(DocumentKind::Drawio, title, text, String::new())
        });
    }
    if lower_name.ends_with(".png") || lower_name.ends_with(".svg") {
        return None;
    }

    // 0b. Modelos ArchiMate: el XML crudo pesa megas (se cortaba al límite y se perdían los
    // nombres); se indexan los nombres, la documentación y las propiedades de sus elementos.
    if lower_name.ends_with(".archimate") {
        if bytes.len() > 64 * 1024 * 1024 {
            return None;
        }
        let (_, decoded) = detect_and_decode(bytes);
        let mut text = super::archimate_extractor::archimate_text(&decoded);
        // Ya es solo texto útil (un modelo de 3 MB deja ~300 mil caracteres): tope propio más alto
        truncate_safe(
            &mut text,
            indexing.max_extracted_chars.max(ARCHIMATE_MAX_CHARS),
        );
        return Some(Extracted::plain(
            DocumentKind::Archimate,
            file_stem.to_string(),
            text,
            String::new(),
        ));
    }

    // 0c. Diagramas de procesos BPMN: los pasos con su tipo y la documentación, sin el dibujo
    if lower_name.ends_with(".bpmn") {
        let (_, decoded) = detect_and_decode(bytes);
        let mut text = super::bpmn_extractor::bpmn_text(&decoded);
        if !text.trim().is_empty() {
            truncate_safe(&mut text, indexing.max_extracted_chars);
            return Some(Extracted::plain(
                DocumentKind::Bpmn,
                file_stem.to_string(),
                text,
                String::new(),
            ));
        }
    }

    // 1. Documentos PDF
    if file_types.is_pdf_file(&lower_name) || lower_name.ends_with(".pdf") {
        if bytes.len() > indexing.max_pdf_file_size {
            return None;
        }
        // Una línea por página: la línea N es la página N
        let (text, truncated) = super::pdf_extractor::extract_pdf_text(
            bytes,
            indexing.max_pdf_pages,
            indexing.max_extracted_chars,
            indexing.max_word_length,
        )
        .ok()?;
        if text.trim().is_empty() {
            return None;
        }
        if truncated {
            return Some(Extracted {
                truncated,
                ..Extracted::plain(
                    DocumentKind::Pdf,
                    file_stem.to_string(),
                    text,
                    String::new(),
                )
            });
        }
        return Some(Extracted::plain(
            DocumentKind::Pdf,
            file_stem.to_string(),
            text,
            String::new(),
        ));
    }

    // 1b. Planillas (Excel, OpenDocument): hojas y filas
    if file_types.is_excel_file(&lower_name) || lower_name.ends_with(".ods") {
        if bytes.len() > indexing.max_docx_file_size {
            return None;
        }
        let sheet =
            super::spreadsheet_extractor::spreadsheet_extract(bytes, indexing.max_extracted_chars)
                .ok()?;
        if sheet.text.trim().is_empty() {
            return None;
        }
        let mut text = sheet.text;
        let truncated = sheet.truncated || text.len() > indexing.max_extracted_chars;
        truncate_safe(&mut text, indexing.max_extracted_chars);
        return Some(Extracted {
            layout: sheet.layout,
            truncated,
            ..Extracted::plain(
                DocumentKind::Excel,
                file_stem.to_string(),
                text,
                String::new(),
            )
        });
    }

    // 2. Documentos Microsoft Word (.docx)
    if lower_name.ends_with(".docx") {
        if bytes.len() > indexing.max_docx_file_size {
            return None;
        }
        // Un párrafo por línea y los títulos aparte (para `hs:` y la sección de cada coincidencia)
        let docx = super::docx_extractor::extract_docx(
            bytes,
            indexing.max_extracted_chars,
            indexing.max_word_length,
        )
        .ok()?;
        if docx.text.trim().is_empty() {
            return None;
        }
        return Some(Extracted {
            truncated: docx.truncated,
            ..Extracted::plain(
                DocumentKind::Word,
                file_stem.to_string(),
                docx.text,
                docx.headings,
            )
        });
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
        return Some(Extracted::plain(
            DocumentKind::Excalidraw,
            file_stem.to_string(),
            extracted_text,
            String::new(),
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
            return Some(Extracted::plain(
                DocumentKind::Excalidraw,
                file_stem.to_string(),
                extracted_text,
                String::new(),
            ));
        }

        let title = Note::title_from_content(&decoded, file_stem);
        let (mut plain_body, mut headings) = extract_markdown_text_and_headings(&decoded);
        truncate_safe(&mut plain_body, indexing.max_extracted_chars);
        truncate_safe(&mut headings, indexing.max_extracted_chars);
        return Some(Extracted::plain(
            DocumentKind::Markdown,
            title,
            plain_body,
            headings,
        ));
    }

    // 7. Diagramas (Mermaid)
    let is_diagram = file_types
        .diagrams
        .iter()
        .any(|ext| lower_name.ends_with(&format!(".{}", ext.to_lowercase())));

    if is_diagram {
        let (_, mut decoded) = detect_and_decode(bytes);
        truncate_safe(&mut decoded, indexing.max_extracted_chars);
        return Some(Extracted::plain(
            DocumentKind::Mermaid,
            file_name.to_string(),
            decoded,
            String::new(),
        ));
    }

    // 8. Código y texto estructurado
    let is_code = file_types
        .code
        .iter()
        .any(|ext| lower_name.ends_with(&format!(".{}", ext.to_lowercase())));

    // JSON y YAML: cada valor con su ruta de claves (si no se pueden interpretar, como código)
    let structured = if lower_name.ends_with(".json") {
        Some(super::structured_extractor::json_text as fn(&str) -> Option<String>)
    } else if lower_name.ends_with(".yaml") || lower_name.ends_with(".yml") {
        Some(super::structured_extractor::yaml_text as fn(&str) -> Option<String>)
    } else {
        None
    };
    if let Some(extract) = structured {
        let (_, decoded) = detect_and_decode(bytes);
        if let Some(mut text) = extract(&decoded).filter(|t| !t.trim().is_empty()) {
            truncate_safe(&mut text, indexing.max_extracted_chars);
            return Some(Extracted::plain(
                DocumentKind::Code,
                file_name.to_string(),
                text,
                String::new(),
            ));
        }
    }

    // XML (también XSD, WSDL, XSL, o un BPMN sin pasos con nombre): solo lo legible, no las etiquetas ni ids o coordenadas
    let is_xml = [".xml", ".xsd", ".wsdl", ".bpmn", ".xsl", ".xslt"]
        .iter()
        .any(|e| lower_name.ends_with(e));
    if is_xml {
        let (_, decoded) = detect_and_decode(bytes);
        let mut text = super::xml_extractor::xml_text(&decoded);
        if !text.trim().is_empty() {
            truncate_safe(&mut text, indexing.max_extracted_chars);
            return Some(Extracted::plain(
                DocumentKind::Code,
                file_name.to_string(),
                text,
                String::new(),
            ));
        }
    }

    if is_code {
        let (_, mut decoded) = detect_and_decode(bytes);
        truncate_safe(&mut decoded, indexing.max_extracted_chars);
        return Some(Extracted::plain(
            DocumentKind::Code,
            file_name.to_string(),
            decoded,
            String::new(),
        ));
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
    extract_markdown_text_and_headings(content).0
}

/// Como [`extract_markdown_plain_text`], pero además devuelve el texto de los títulos y
/// subtítulos (uno por línea) en la misma pasada del parser.
pub fn extract_markdown_text_and_headings(content: &str) -> (String, String) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let parser = Parser::new_ext(content, options);
    let mut plain = String::with_capacity(content.len());
    let mut headings = String::new();
    let mut in_heading = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                headings.push('\n');
                plain.push('\n');
            }
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) | Event::DisplayMath(t) => {
                plain.push_str(&t);
                if in_heading {
                    headings.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak if in_heading => {
                plain.push(' ');
                headings.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => {
                plain.push(' ');
            }
            Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Item)
            | Event::End(TagEnd::TableCell)
            | Event::End(TagEnd::CodeBlock)
            | Event::End(TagEnd::MetadataBlock(_)) => {
                plain.push('\n');
            }
            _ => {}
        }
    }

    (plain, headings)
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
        let (kind, title, body, _) = res.unwrap();
        assert_eq!(kind, DocumentKind::Markdown);
        assert_eq!(title, "Mi Título");
        assert!(body.contains("Este es un párrafo con negrita y enlace."));
        assert!(body.contains("let x = 42;"));
    }

    #[test]
    fn test_extract_markdown_headings() {
        let md = "# Plan *anual*\n\nIntro con texto.\n\n## Presupuesto\n\nCifras.\n\n### Gastos `fijos`\n\nDetalle.";
        let file_types = SupportedFileTypes::default();
        let (kind, _, body, headings) =
            extract(Path::new("plan.md"), md.as_bytes(), &file_types).unwrap();
        assert_eq!(kind, DocumentKind::Markdown);
        assert_eq!(
            headings.lines().collect::<Vec<_>>(),
            vec!["Plan anual", "Presupuesto", "Gastos fijos"]
        );
        // El cuerpo conserva el texto de los encabezados
        assert!(body.contains("Presupuesto"));
        assert!(!headings.contains("Intro"));
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
        let (kind, title, body, _) = res.unwrap();
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
        let (kind, title, body, _) = res.unwrap();
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
        let (kind, title, body, _) = res.unwrap();
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
    fn test_extract_sql_file() {
        let sql = "SELECT id, nombre FROM clientes WHERE activo = TRUE;";
        let file_types = SupportedFileTypes::default();
        let (kind, title, body, _) =
            extract(Path::new("consultas/clientes.sql"), sql.as_bytes(), &file_types).unwrap();
        assert_eq!(kind, DocumentKind::Code);
        assert_eq!(title, "clientes.sql");
        assert!(body.contains("FROM clientes"));
    }

    #[test]
    fn test_extract_cobol_file() {
        let cobol_src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO-WORLD.\n       PROCEDURE DIVISION.\n           DISPLAY 'HELLO COBOL'.\n           STOP RUN.";
        let file_types = SupportedFileTypes::default();
        let path = Path::new("subfolder/hello.cbl");
        let res = extract(path, cobol_src.as_bytes(), &file_types);
        assert!(res.is_some());
        let (kind, title, body, _) = res.unwrap();
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
        let (kind, title, body, _) = res.unwrap();
        assert_eq!(kind, DocumentKind::Word);
        assert_eq!(title, "reporte");
        assert!(body.contains("Contenido extraído del archivo Word"));
    }

    #[test]
    fn test_extract_pdf_file() {
        use lopdf::content::Content;
        use lopdf::{Document, Object, Stream, dictionary};

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
                lopdf::content::Operation::new(
                    "Tj",
                    vec![Object::string_literal("Texto indexable de PDF")],
                ),
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
        let (kind, title, body, _) = res.unwrap();
        assert_eq!(kind, DocumentKind::Pdf);
        assert_eq!(title, "manual");
        assert!(body.contains("Texto indexable de PDF"));
    }
}
