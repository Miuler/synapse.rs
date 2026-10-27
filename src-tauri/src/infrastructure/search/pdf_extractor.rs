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

/// Extrae información detallada de un PDF: páginas totales, metadatos y texto por páginas.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PdfPageSnippet {
    pub page: u32,
    pub text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PdfInspectionResult {
    pub total_pages: usize,
    pub pages: Vec<PdfPageSnippet>,
}

/// Extrae texto acotado entre `start_page` y `end_page` (base 1, inclusive).
pub fn extract_pdf_page_range(
    bytes: &[u8],
    start_page: usize,
    end_page: usize,
    max_chars_per_page: usize,
) -> Result<PdfInspectionResult, String> {
    let doc = Document::load_mem(bytes)
        .map_err(|e| format!("Error al decodificar PDF: {e}"))?;

    let pages_map = doc.get_pages();
    let total_pages = pages_map.len();
    let mut sorted_page_nums: Vec<u32> = pages_map.keys().copied().collect();
    sorted_page_nums.sort_unstable();

    let start = start_page.max(1);
    let end = end_page.min(total_pages).max(start);

    let mut snippets = Vec::new();

    for &page_num in sorted_page_nums.iter() {
        let p_idx = page_num as usize;
        if p_idx >= start && p_idx <= end {
            let page_text = doc.extract_text(&[page_num]).unwrap_or_default();
            let mut cleaned = String::with_capacity(page_text.len().min(max_chars_per_page));
            for word in page_text.split_whitespace() {
                if word.len() > 80 { continue; }
                if !cleaned.is_empty() { cleaned.push(' '); }
                cleaned.push_str(word);
                if cleaned.len() >= max_chars_per_page {
                    cleaned.truncate(max_chars_per_page);
                    break;
                }
            }
            snippets.push(PdfPageSnippet {
                page: page_num,
                text: cleaned,
            });
        }
    }

    Ok(PdfInspectionResult {
        total_pages,
        pages: snippets,
    })
}

/// Busca una palabra o frase clave dentro de todas las páginas del PDF y devuelve las coincidencias con snippets.
pub fn search_pdf_content(
    bytes: &[u8],
    query: &str,
    max_results: usize,
) -> Result<Vec<PdfPageSnippet>, String> {
    let doc = Document::load_mem(bytes)
        .map_err(|e| format!("Error al decodificar PDF: {e}"))?;

    let query_lower = query.to_lowercase();
    let pages_map = doc.get_pages();
    let mut sorted_page_nums: Vec<u32> = pages_map.keys().copied().collect();
    sorted_page_nums.sort_unstable();

    let mut matches = Vec::new();

    for &page_num in sorted_page_nums.iter() {
        let page_text = doc.extract_text(&[page_num]).unwrap_or_default();
        if page_text.to_lowercase().contains(&query_lower) {
            // Generar un snippet contextual
            let snippet = page_text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let truncated = if snippet.len() > 300 {
                format!("{}...", &snippet[..300])
            } else {
                snippet
            };
            matches.push(PdfPageSnippet {
                page: page_num,
                text: truncated,
            });
            if matches.len() >= max_results {
                break;
            }
        }
    }

    Ok(matches)
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
