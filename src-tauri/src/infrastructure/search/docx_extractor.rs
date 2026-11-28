use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::collections::HashSet;
use std::io::{Cursor, Read};

/// Texto de un `.docx` para el índice.
pub struct DocxText {
    /// Un párrafo con texto por línea (los vacíos no cuentan): la línea N es el párrafo N.
    pub text: String,
    /// Los párrafos que son títulos (estilo de título o nivel de esquema), uno por línea.
    pub headings: String,
    /// Se cortó por el límite de caracteres.
    pub truncated: bool,
}

/// Texto plano de un `.docx`: un párrafo por línea (ver [`extract_docx`]).
pub fn extract_docx_text(
    bytes: &[u8],
    max_extracted_chars: usize,
    max_word_length: usize,
) -> Result<String, String> {
    extract_docx(bytes, max_extracted_chars, max_word_length).map(|d| d.text)
}

/// Extrae el texto de un documento Microsoft Word (.docx) mediante streaming: un párrafo por
/// línea y, aparte, los títulos (para filtrar con `hs:` y saber en qué sección está cada
/// coincidencia).
///
/// Utiliza `zip` para descomprimir `word/document.xml` (y `word/styles.xml`, para saber qué
/// estilos son títulos) y `quick-xml` para extraer los nodos de texto `<w:t>`.
pub fn extract_docx(
    bytes: &[u8],
    max_extracted_chars: usize,
    max_word_length: usize,
) -> Result<DocxText, String> {
    let reader = Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| format!("Failed to open docx file as ZIP: {e}"))?;

    let heading_styles = zip
        .by_name("word/styles.xml")
        .ok()
        .and_then(|mut f| {
            let mut xml = String::new();
            f.read_to_string(&mut xml).ok().map(|_| xml)
        })
        .map(|xml| heading_style_ids(&xml))
        .unwrap_or_default();

    let mut xml_file = zip
        .by_name("word/document.xml")
        .map_err(|e| format!("'word/document.xml' not found in docx file: {e}"))?;

    let mut xml_content = String::new();
    xml_file
        .read_to_string(&mut xml_content)
        .map_err(|e| format!("Failed to read docx XML content: {e}"))?;

    // Sin `trim_text`: los espacios al borde de un run separan palabras
    let mut xml_reader = Reader::from_str(&xml_content);

    let mut out = DocxText {
        text: String::with_capacity(xml_content.len().min(max_extracted_chars)),
        headings: String::new(),
        truncated: false,
    };
    // Una palabra puede venir repartida en varios runs y las entidades (`&amp;`…) son eventos aparte:
    // se junta el texto del párrafo antes de separarlo en palabras
    let mut paragraph = String::new();
    let mut inside_t = false;
    let mut is_heading = false;

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
                "w:pStyle" => {
                    if let Some(id) = val(&e) {
                        is_heading |= heading_styles.contains(&id) || looks_like_heading_id(&id);
                    }
                }
                "w:outlineLvl" => {
                    is_heading |= val(&e)
                        .and_then(|v| v.parse::<u8>().ok())
                        .is_some_and(|l| l < 9)
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == "w:p" => {
                let full = !append_paragraph(
                    &mut out,
                    &paragraph,
                    is_heading,
                    max_extracted_chars,
                    max_word_length,
                );
                paragraph.clear();
                is_heading = false;
                if full {
                    return Ok(out);
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

    // Lo que quedara si el XML estaba cortado
    append_paragraph(
        &mut out,
        &paragraph,
        is_heading,
        max_extracted_chars,
        max_word_length,
    );
    Ok(out)
}

/// Atributo `w:val`.
fn val(e: &BytesStart) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == "w:val")
        .and_then(|a| {
            quick_xml::escape::unescape(&a.value)
                .ok()
                .map(|v| v.into_owned())
        })
}

/// Ids de los estilos de párrafo que son títulos: su nombre interno es `heading N` o `Title`
/// (también en un Word en español, donde se muestran como «Título N») o tienen nivel de esquema.
fn heading_style_ids(styles_xml: &str) -> HashSet<String> {
    let mut reader = Reader::from_str(styles_xml);
    let mut ids = HashSet::new();
    let mut current: Option<String> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => match e.name().as_ref() {
                "w:style" => {
                    let attr = |k: &str| {
                        e.attributes()
                            .flatten()
                            .find(|a| a.key.as_ref() == k)
                            .and_then(|a| {
                                quick_xml::escape::unescape(&a.value)
                                    .ok()
                                    .map(|v| v.into_owned())
                            })
                    };
                    current = (attr("w:type").as_deref() == Some("paragraph"))
                        .then(|| attr("w:styleId"))
                        .flatten();
                }
                "w:name" => {
                    if let (Some(id), Some(name)) = (&current, val(&e)) {
                        let name = name.to_lowercase();
                        if name.starts_with("heading") || name == "title" {
                            ids.insert(id.clone());
                        }
                    }
                }
                "w:outlineLvl" => {
                    if let Some(id) = &current
                        && val(&e)
                            .and_then(|v| v.parse::<u8>().ok())
                            .is_some_and(|l| l < 9)
                    {
                        ids.insert(id.clone());
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == "w:style" => current = None,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    ids
}

/// Sin `styles.xml`, por el id del estilo (`Heading1`, `Ttulo1` en Word en español, `Title`).
fn looks_like_heading_id(id: &str) -> bool {
    let id = id.to_lowercase();
    id.starts_with("heading")
        || id.starts_with("ttulo")
        || id.starts_with("titulo")
        || id == "title"
}

/// Añade el párrafo como una línea (sus palabras separadas por un espacio; los vacíos no
/// cuentan). `false` si se llegó al límite (se corta sin partir un carácter).
fn append_paragraph(
    out: &mut DocxText,
    source: &str,
    is_heading: bool,
    max_chars: usize,
    max_word_length: usize,
) -> bool {
    // Descartar tokens anormalmente largos (evita basura de fuentes o hashes en Tantivy)
    let line = source
        .split_whitespace()
        .filter(|w| w.len() <= max_word_length)
        .collect::<Vec<_>>()
        .join(" ");
    if line.is_empty() {
        return true;
    }
    if !out.text.is_empty() {
        out.text.push('\n');
    }
    out.text.push_str(&line);
    if is_heading {
        out.headings.push_str(&line);
        out.headings.push('\n');
    }
    if out.text.len() >= max_chars {
        super::content_extractor::truncate_safe(&mut out.text, max_chars);
        out.truncated = true;
        return false;
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
            "Hola mundo\notro párrafo más"
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
            extract_docx_text(
                &docx("<w:p><w:r><w:t>ñáé 🧠 ü</w:t></w:r></w:p>"),
                limit,
                80,
            )
            .unwrap();
        }
    }

    #[test]
    fn un_parrafo_por_linea_y_titulos_aparte() {
        let bytes = docx(concat!(
            r#"<w:p><w:pPr><w:pStyle w:val="Ttulo1"/></w:pPr><w:r><w:t>1. Contrato</w:t></w:r></w:p>"#,
            r#"<w:p><w:r><w:t>Primer párrafo</w:t></w:r></w:p>"#,
            r#"<w:p></w:p>"#,
            r#"<w:p><w:pPr><w:outlineLvl w:val="1"/></w:pPr><w:r><w:t>1.1 Salida</w:t></w:r></w:p>"#,
            r#"<w:p><w:r><w:t>TIEL / BCTITPE8</w:t></w:r></w:p>"#,
        ));
        let d = extract_docx(&bytes, 200_000, 80).unwrap();
        assert_eq!(
            d.text, "1. Contrato\nPrimer párrafo\n1.1 Salida\nTIEL / BCTITPE8",
            "el párrafo vacío no cuenta"
        );
        assert_eq!(d.headings, "1. Contrato\n1.1 Salida\n");
        assert!(!d.truncated);
        assert!(extract_docx(&bytes, 15, 80).unwrap().truncated);
    }

    #[test]
    fn estilos_de_titulo_desde_styles_xml() {
        let styles = r#"<w:styles xmlns:w="w">
            <w:style w:type="paragraph" w:styleId="MiTitulo"><w:name w:val="heading 2"/></w:style>
            <w:style w:type="paragraph" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
            <w:style w:type="paragraph" w:styleId="Capitulo"><w:name w:val="Capítulo"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>
        </w:styles>"#;
        let ids = heading_style_ids(styles);
        assert!(ids.contains("MiTitulo") && ids.contains("Capitulo") && !ids.contains("Normal"));
    }
}
