use lopdf::Document;

/// Extrae texto plano de un documento PDF aplicando límites de páginas, caracteres y longitud de palabra.
///
/// Utiliza `lopdf` en pure Rust para decodificar los streams `/Contents`
/// de las primeras `max_pages` páginas del documento sin depender de bibliotecas C del sistema.
pub fn extract_pdf_text(
    bytes: &[u8],
    max_pages: usize,
    max_extracted_chars: usize,
    max_word_length: usize,
) -> Result<String, String> {
    let doc = Document::load_mem(bytes)
        .map_err(|e| format!("Error al decodificar PDF: {e}"))?;

    let pages = doc.get_pages();
    let page_numbers: Vec<u32> = pages.keys().copied().take(max_pages).collect();

    if page_numbers.is_empty() {
        return Ok(String::new());
    }

    let raw_text = doc
        .extract_text(&page_numbers)
        .map_err(|e| format!("Error al extraer texto de páginas del PDF: {e}"))?;

    let mut text = String::with_capacity(raw_text.len().min(max_extracted_chars));
    for word in raw_text.split_whitespace() {
        // Descartar palabras excesivamente largas (como hashes o basura de tipografías corruptas)
        if word.len() > max_word_length {
            continue;
        }

        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(word);

        if text.len() >= max_extracted_chars {
            text.truncate(max_extracted_chars);
            break;
        }
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;
    use lopdf::{dictionary, Object, Stream};

    #[test]
    fn test_extract_pdf_basic() {
        // Construir un PDF mínimo sintético en memoria con lopdf
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });

        let content = Content {
            operations: vec![
                lopdf::content::Operation::new("BT", vec![]),
                lopdf::content::Operation::new("Tf", vec!["F1".into(), 12.into()]),
                lopdf::content::Operation::new("Tj", vec![Object::string_literal("Documento PDF de Prueba")]),
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

        let extracted = extract_pdf_text(&buffer, 10, 200_000, 80).unwrap();
        assert!(extracted.contains("Documento"));
        assert!(extracted.contains("PDF"));
        assert!(extracted.contains("Prueba"));
    }
}
