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

    // Sin `trim_text`: los espacios al borde de un run separan palabras
    let mut xml_reader = Reader::from_str(&xml_content);

    let mut text = String::with_capacity(xml_content.len().min(max_extracted_chars));
    // Una palabra puede venir repartida en varios runs y las entidades (`&amp;`…) son eventos aparte:
    // se junta el texto del párrafo antes de separarlo en palabras
    let mut paragraph = String::new();
    let mut inside_t = false;

    loop {
        match xml_reader.read_event() {
            Ok(Event::Start(e)) if e.name().as_ref() == "w:t" => inside_t = true,
            Ok(Event::End(e)) if e.name().as_ref() == "w:t" => inside_t = false,
            Ok(Event::Text(e)) if inside_t => {
                let s: &str = &e;
                paragraph.push_str(s);
            }
            Ok(Event::CData(e)) if inside_t => {
                let s: &str = &e;
                paragraph.push_str(s);
            }
            Ok(Event::GeneralRef(r)) if inside_t => {
                let name: &str = &r;
                paragraph.push(match name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    _ => r.resolve_char_ref().ok().flatten().unwrap_or(' '),
                });
            }
            Ok(Event::Start(e) | Event::Empty(e)) => match e.name().as_ref() {
                "w:tab" | "w:br" | "w:cr" => paragraph.push(' '),
                "w:noBreakHyphen" => paragraph.push('-'),
                // Un cuadro de texto abre párrafos dentro de otro
                "w:p" => paragraph.push(' '),
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == "w:p" => {
                if !append_words(&mut text, &paragraph, max_extracted_chars, max_word_length) {
                    return Ok(text);
                }
                paragraph.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("[DocxExtractor] Advertencia al parsear XML: {e}");
                break;
            }
            _ => {}
        }
    }

    // Lo que quedara si el XML estaba cortado
    append_words(&mut text, &paragraph, max_extracted_chars, max_word_length);
    Ok(text)
}

/// Añade las palabras de `source` a `text`. `false` si se llegó al límite (se corta sin partir un carácter).
fn append_words(text: &mut String, source: &str, max_chars: usize, max_word_length: usize) -> bool {
    for word in source.split_whitespace() {
        // Descartar tokens anormalmente largos (evita basura de fuentes o hashes en Tantivy)
        if word.len() > max_word_length {
            continue;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(word);
        if text.len() >= max_chars {
            super::content_extractor::truncate_safe(text, max_chars);
            return false;
        }
    }
    true
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
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
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

    /// Un `.docx` mínimo con ese contenido en `w:body`.
    fn docx(body: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        zip.start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        let xml = format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
        );
        zip.write_all(xml.as_bytes()).unwrap();
        zip.finish().unwrap();
        buffer
    }

    #[test]
    fn une_las_palabras_repartidas_en_varios_runs() {
        let bytes = docx(concat!(
            r#"<w:p><w:r><w:t>Ho</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>la</w:t></w:r>"#,
            r#"<w:r><w:t xml:space="preserve"> mundo</w:t></w:r></w:p>"#,
            r#"<w:p><w:r><w:t>otro</w:t><w:tab/><w:t>párrafo</w:t><w:br/><w:t>más</w:t></w:r></w:p>"#,
        ));
        assert_eq!(
            extract_docx_text(&bytes, 200_000, 80).unwrap(),
            "Hola mundo otro párrafo más"
        );
    }

    #[test]
    fn conserva_las_entidades() {
        let bytes = docx(
            "<w:p><w:r><w:t>I&amp;D &lt;b&gt; &quot;x&quot; a&#241;o ca&#xF1;a <![CDATA[y & z]]></w:t></w:r></w:p>",
        );
        assert_eq!(
            extract_docx_text(&bytes, 200_000, 80).unwrap(),
            "I&D <b> \"x\" año caña y & z"
        );
    }

    #[test]
    fn el_limite_no_parte_un_caracter() {
        let bytes = docx("<w:p><w:r><w:t>aaaaaaaaañ</w:t></w:r></w:p>");
        assert_eq!(extract_docx_text(&bytes, 10, 80).unwrap(), "aaaaaaaaa");
        for limit in 0..12 {
            extract_docx_text(&docx("<w:p><w:r><w:t>ñáé 🧠 ü</w:t></w:r></w:p>"), limit, 80).unwrap();
        }
    }
}
