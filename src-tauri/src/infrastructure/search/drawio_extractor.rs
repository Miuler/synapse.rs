//! Texto de un diagrama de draw.io guardado dentro de una imagen (para el índice full-text).
//!
//! draw.io guarda el diagrama `mxfile` en el PNG (bloque `tEXt`/`zTXt`/`iTXt`, codificado con
//! `encodeURIComponent`) o en el atributo `content` del SVG. Cada `<diagram>` trae el modelo en XML
//! o comprimido (base64 de deflate sin cabecera del XML codificado como URI). Se indexan los textos
//! de las figuras (`value`/`label`, sin su HTML) y los nombres de las páginas.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use flate2::read::{DeflateDecoder, ZlibDecoder};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::Read;

const PNG_SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// ¿El archivo (PNG o SVG) lleva un diagrama de draw.io? Lee lo mínimo: en un PNG, solo las
/// cabeceras de sus bloques hasta los datos de la imagen; en un SVG, el comienzo del archivo.
pub fn file_has_drawio(path: &std::path::Path) -> bool {
    use crate::domain::vault::drawio_image::svg_has_drawio_diagram;
    use std::io::{Seek, SeekFrom};
    let lower = path.to_string_lossy().to_lowercase();
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if lower.ends_with(".svg") {
        let mut head = Vec::new();
        return (&mut file).take(256 * 1024).read_to_end(&mut head).is_ok() && svg_has_drawio_diagram(&head);
    }
    if !lower.ends_with(".png") {
        return false;
    }
    let mut sig = [0u8; 8];
    if file.read_exact(&mut sig).is_err() || sig != PNG_SIGNATURE {
        return false;
    }
    // Cada bloque: largo (4), tipo (4), datos, CRC (4); de los de texto basta la clave
    loop {
        let mut header = [0u8; 8];
        if file.read_exact(&mut header).is_err() {
            return false;
        }
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as u64;
        match &header[4..8] {
            b"IDAT" | b"IEND" => return false,
            b"tEXt" | b"zTXt" | b"iTXt" if len >= 7 => {
                let mut key = [0u8; 7];
                if file.read_exact(&mut key).is_err() {
                    return false;
                }
                if &key == b"mxfile\0" {
                    return true;
                }
                if file.seek(SeekFrom::Current(len as i64 - 7 + 4)).is_err() {
                    return false;
                }
            }
            _ => {
                if file.seek(SeekFrom::Current(len as i64 + 4)).is_err() {
                    return false;
                }
            }
        }
    }
}

/// El `mxfile` (XML) guardado en un PNG de draw.io.
pub fn png_mxfile(bytes: &[u8]) -> Option<String> {
    if !bytes.starts_with(PNG_SIGNATURE) {
        return None;
    }
    let mut pos = PNG_SIGNATURE.len();
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().ok()?) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let start = pos + 8;
        let end = start.checked_add(len).filter(|e| *e <= bytes.len())?;
        let data = &bytes[start..end];
        let key_end = data.iter().position(|b| *b == 0);
        if let Some(k) = key_end.filter(|k| &data[..*k] == b"mxfile") {
            let rest = &data[k + 1..];
            let raw = match kind {
                b"tEXt" => Some(rest.iter().map(|b| *b as char).collect::<String>()),
                b"zTXt" => inflate_zlib(rest.get(1..)?),
                b"iTXt" => {
                    // flag de compresión, método, idioma\0, palabra traducida\0, texto
                    let compressed = *rest.first()? == 1;
                    let mut i = 2;
                    for _ in 0..2 {
                        i += rest.get(i..)?.iter().position(|b| *b == 0)? + 1;
                    }
                    let text = rest.get(i..)?;
                    if compressed { inflate_zlib(text) } else { String::from_utf8(text.to_vec()).ok() }
                }
                _ => None,
            }?;
            return Some(percent_decode(&raw));
        }
        if kind == b"IDAT" || kind == b"IEND" {
            return None;
        }
        pos = end + 4;
    }
    None
}

/// El `mxfile` (XML) guardado en el atributo `content` del `<svg>` de draw.io.
pub fn svg_mxfile(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let start = text.find("<svg")?;
    let tag_end = text[start..].find('>').map_or(text.len(), |e| start + e);
    let tag = &text[start..tag_end];
    let attr = tag.find("content=")? + "content=".len();
    let quote = tag[attr..].chars().next()?;
    let value_start = attr + 1;
    let value_end = value_start + tag[value_start..].find(quote)?;
    let value = quick_xml::escape::unescape(&tag[value_start..value_end]).ok()?.into_owned();
    value.contains("mxfile").then_some(value)
}

/// Textos de las figuras y nombres de las páginas de un `mxfile`, separados por líneas.
pub fn mxfile_text(xml: &str) -> String {
    let mut out = Vec::new();
    collect(xml, &mut out, 0);
    out.join("\n")
}

fn collect(xml: &str, out: &mut Vec<String>, depth: u8) {
    if depth > 2 {
        return;
    }
    let mut reader = Reader::from_str(xml);
    let mut in_diagram = false;
    let mut diagram_text = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let is_diagram = e.name().as_ref() == "diagram";
                for attr in e.attributes().flatten() {
                    let key = attr.key.as_ref();
                    let wanted = matches!(key, "value" | "label") || (is_diagram && key == "name");
                    if !wanted {
                        continue;
                    }
                    #[allow(deprecated)] // `normalized_value` pide la versión de XML; aquí basta con deshacer las entidades
                    if let Ok(v) = attr.unescape_value() {
                        let clean = strip_html(&v);
                        if !clean.is_empty() {
                            out.push(clean);
                        }
                    }
                }
                if is_diagram {
                    in_diagram = true;
                    diagram_text.clear();
                }
            }
            Ok(Event::Text(t)) if in_diagram => {
                let s: &str = &t;
                diagram_text.push_str(s);
            }
            Ok(Event::End(e)) if e.name().as_ref() == "diagram" => {
                in_diagram = false;
                // Diagrama comprimido: base64 → deflate sin cabecera → URI → XML
                let packed = diagram_text.trim();
                if !packed.is_empty()
                    && let Some(model) = decompress_diagram(packed)
                {
                    collect(&model, out, depth + 1);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

fn decompress_diagram(packed: &str) -> Option<String> {
    let bytes = BASE64.decode(packed).ok()?;
    let mut inflated = String::new();
    DeflateDecoder::new(&bytes[..]).read_to_string(&mut inflated).ok()?;
    Some(percent_decode(&inflated))
}

fn inflate_zlib(bytes: &[u8]) -> Option<String> {
    let mut s = String::new();
    ZlibDecoder::new(bytes).read_to_string(&mut s).ok()?;
    Some(s)
}

/// Deshace `encodeURIComponent` (los `%XX` en UTF-8).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Quita las etiquetas HTML de un rótulo y deshace las entidades más comunes.
fn strip_html(s: &str) -> String {
    let mut text = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => {
                in_tag = true;
                text.push(' ');
            }
            '>' if in_tag => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    const MODEL: &str = r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="2" value="&lt;b&gt;Servicio&lt;/b&gt;&amp;nbsp;Pagos" vertex="1"/><object id="3" label="Base de datos"><mxCell vertex="1"/></object></root></mxGraphModel>"#;

    fn encode_uri(s: &str) -> String {
        s.bytes()
            .map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") })
            .collect()
    }

    fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut c = (data.len() as u32).to_be_bytes().to_vec();
        c.extend_from_slice(kind);
        c.extend_from_slice(data);
        c.extend_from_slice(&[0; 4]);
        c
    }

    #[test]
    fn texto_de_un_diagrama_sin_comprimir() {
        let xml = format!(r#"<mxfile><diagram name="Página 1">{MODEL}</diagram></mxfile>"#);
        assert_eq!(mxfile_text(&xml), "Página 1\nServicio Pagos\nBase de datos");
    }

    #[test]
    fn texto_de_un_diagrama_comprimido_dentro_de_un_png() {
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(encode_uri(MODEL).as_bytes()).unwrap();
        let packed = BASE64.encode(enc.finish().unwrap());
        let mxfile = format!(r#"<mxfile><diagram name="Flujo">{packed}</diagram></mxfile>"#);
        let mut text = b"mxfile\0".to_vec();
        text.extend_from_slice(encode_uri(&mxfile).as_bytes());
        let mut png = PNG_SIGNATURE.to_vec();
        png.extend(png_chunk(b"IHDR", &[0; 13]));
        png.extend(png_chunk(b"tEXt", &text));
        png.extend(png_chunk(b"IDAT", &[1]));
        let xml = png_mxfile(&png).expect("mxfile");
        assert_eq!(mxfile_text(&xml), "Flujo\nServicio Pagos\nBase de datos");
    }

    #[test]
    fn mxfile_dentro_de_un_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" content="&lt;mxfile&gt;&lt;diagram name=&quot;A&quot;&gt;&lt;/diagram&gt;&lt;/mxfile&gt;"><rect/></svg>"#;
        assert_eq!(mxfile_text(&svg_mxfile(svg.as_bytes()).unwrap()), "A");
        assert!(svg_mxfile(b"<svg><rect/></svg>").is_none());
    }
}
