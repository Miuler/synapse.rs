use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Cursor, Read};

/// Extrae texto plano de un documento Microsoft Word (.docx) mediante streaming.
///
/// Utiliza `zip` para descomprimir únicamente el stream de `word/document.xml`
/// y `quick-xml` para extraer los nodos de texto `<w:t>` de forma ultra-rápida.
pub fn extract_docx_text(
    bytes: &[u8],
    max_extracted_chars: usize,
    max_word_length: usize,
) -> Result<String, String> {
    let reader = Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| format!("Error al abrir archivo docx como ZIP: {e}"))?;

    let mut xml_file = zip
        .by_name("word/document.xml")
        .map_err(|e| format!("No se encontró 'word/document.xml' en el archivo docx: {e}"))?;

    let mut xml_content = String::new();
    xml_file
        .read_to_string(&mut xml_content)
        .map_err(|e| format!("Error al leer contenido XML de docx: {e}"))?;

    let mut xml_reader = Reader::from_str(&xml_content);
    xml_reader.config_mut().trim_text(true);

    let mut text = String::with_capacity(xml_content.len().min(max_extracted_chars));
    let mut inside_t = false;

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) if e.name().as_ref() == b"w:t" => {
                inside_t = true;
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"w:t" => {
                inside_t = false;
            }
            Ok(Event::Text(e)) if inside_t => {
                if let Ok(unescaped) = e.unescape() {
                    for word in unescaped.split_whitespace() {
                        // Descartar tokens anormalmente largos (evita basura de fuentes o hashes en Tantivy)
                        if word.len() > max_word_length {
                            continue;
                        }

                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(word);

                        if text.len() >= max_extracted_chars {
                            text.truncate(max_extracted_chars);
                            return Ok(text);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("[DocxExtractor] Advertencia al parsear XML: {e}");
                break;
            }
            _ => {}
        }
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    #[test]
    fn test_extract_docx_basic() {
        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("word/document.xml", options).unwrap();

            let xml_content = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:body>
                    <w:p>
                        <w:r>
                            <w:t>Hola mundo desde Synapse</w:t>
                        </w:r>
                    </w:p>
                    <w:p>
                        <w:r>
                            <w:t>Segunda línea del documento Word</w:t>
                        </w:r>
                    </w:p>
                </w:body>
            </w:document>"#;
            zip.write_all(xml_content.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        let extracted = extract_docx_text(&buffer, 200_000, 80).unwrap();
        assert!(extracted.contains("Hola mundo desde Synapse"));
        assert!(extracted.contains("Segunda línea del documento Word"));
    }

    #[test]
    fn test_extract_docx_limits_and_word_filtering() {
        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default();
            zip.start_file("word/document.xml", options).unwrap();

            let garbage_word = "A".repeat(100); // 100 caracteres > max 80
            let xml_content = format!(
                r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                    <w:body>
                        <w:p>
                            <w:t>PalabraValida {garbage_word} OtraPalabraValida</w:t>
                        </w:p>
                    </w:body>
                </w:document>"#
            );
            zip.write_all(xml_content.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        // Filtro de palabra máxima de 80 chars
        let extracted = extract_docx_text(&buffer, 200_000, 80).unwrap();
        assert!(extracted.contains("PalabraValida"));
        assert!(extracted.contains("OtraPalabraValida"));
        assert!(!extracted.contains(&"A".repeat(81)));

        // Límite de caracteres máximos
        let truncated = extract_docx_text(&buffer, 10, 80).unwrap();
        assert_eq!(truncated.len(), 10);
    }
}
