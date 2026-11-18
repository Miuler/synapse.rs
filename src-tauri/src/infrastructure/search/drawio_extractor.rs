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

/// Una página de un diagrama de draw.io, ordenada para leerla: contenedores con sus figuras,
/// figuras sueltas y conexiones.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct DrawioPage {
    pub name: String,
    pub containers: Vec<DrawioContainer>,
    /// Figuras con texto que no están dentro de un contenedor.
    pub labels: Vec<String>,
    pub edges: Vec<DrawioEdge>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct DrawioContainer {
    pub title: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct DrawioEdge {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub label: String,
}

/// Estructura de cada página de un `mxfile`: qué figura está dentro de qué contenedor y qué
/// conecta con qué (las conexiones se nombran con el texto de sus extremos).
pub fn mxfile_structure(xml: &str) -> Vec<DrawioPage> {
    let mut pages = Vec::new();
    for (name, model) in diagram_models(xml) {
        pages.push(page_structure(name, &model));
    }
    pages
}

#[derive(Default)]
struct Cell {
    label: String,
    parent: String,
    source: String,
    target: String,
    edge: bool,
    /// x, y, ancho, alto (relativos a su padre)
    geometry: Option<(f64, f64, f64, f64)>,
}

/// `(nombre, XML del modelo)` de cada `<diagram>`, descomprimido si hace falta.
fn diagram_models(xml: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut reader = Reader::from_str(xml);
    let mut current: Option<(String, usize)> = None;
    let mut text = String::new();
    loop {
        let pos = reader.buffer_position() as usize;
        match reader.read_event() {
            Ok(Event::Start(e)) if e.name().as_ref() == "diagram" => {
                let name = attr_of(&e, "name").unwrap_or_default();
                current = Some((name, reader.buffer_position() as usize));
                text.clear();
            }
            Ok(Event::Text(t)) if current.is_some() => {
                let s: &str = &t;
                text.push_str(s);
            }
            Ok(Event::End(e)) if e.name().as_ref() == "diagram" => {
                if let Some((name, start)) = current.take() {
                    let inner = &xml[start..pos];
                    let model = if inner.contains("<mxGraphModel") {
                        Some(inner.to_string())
                    } else {
                        decompress_diagram(text.trim())
                    };
                    if let Some(m) = model {
                        out.push((name, m));
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out
}

fn page_structure(name: String, model: &str) -> DrawioPage {
    let mut cells: Vec<(String, Cell)> = Vec::new();
    let mut reader = Reader::from_str(model);
    // `<object label=…>` envuelve al `<mxCell>` que tiene parent/source/target
    let mut object: Option<(String, String)> = None;
    // `<mxGeometry>` va dentro del `<mxCell>` abierto
    let mut open_cell: Option<usize> = None;
    loop {
        match reader.read_event() {
            Ok(Event::End(e)) if e.name().as_ref() == "mxCell" => open_cell = None,
            Ok(ev @ Event::Start(_)) | Ok(ev @ Event::Empty(_)) => {
                let is_start = matches!(ev, Event::Start(_));
                let (Event::Start(e) | Event::Empty(e)) = ev else { continue };
                let tag = e.name();
                match tag.as_ref() {
                    "mxGeometry" => {
                        if let Some(i) = open_cell {
                            let num = |k: &str| attr_of(&e, k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                            cells[i].1.geometry = Some((num("x"), num("y"), num("width"), num("height")));
                        }
                    }
                    "object" | "UserObject" => {
                        object = Some((
                            attr_of(&e, "id").unwrap_or_default(),
                            attr_of(&e, "label").map(|l| strip_html(&l)).unwrap_or_default(),
                        ));
                    }
                    "mxCell" => {
                        let (id, label) = match object.take() {
                            Some((id, label)) => (id, label),
                            None => (
                                attr_of(&e, "id").unwrap_or_default(),
                                attr_of(&e, "value").map(|v| strip_html(&v)).unwrap_or_default(),
                            ),
                        };
                        cells.push((
                            id,
                            Cell {
                                label,
                                parent: attr_of(&e, "parent").unwrap_or_default(),
                                source: attr_of(&e, "source").unwrap_or_default(),
                                target: attr_of(&e, "target").unwrap_or_default(),
                                edge: attr_of(&e, "edge").as_deref() == Some("1"),
                                geometry: None,
                            },
                        ));
                        if is_start {
                            open_cell = Some(cells.len() - 1);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    let index: std::collections::HashMap<&str, usize> = cells.iter().enumerate().map(|(i, (id, _))| (id.as_str(), i)).collect();
    // Texto de una figura; si no tiene, el de su primer hijo con texto (p. ej. una caja con rótulo aparte)
    let label_of = |id: &str| -> String {
        let Some(&i) = index.get(id) else { return String::new() };
        if !cells[i].1.label.is_empty() {
            return cells[i].1.label.clone();
        }
        cells.iter().find(|(_, c)| c.parent == id && !c.edge && !c.label.is_empty()).map(|(_, c)| c.label.clone()).unwrap_or_default()
    };
    // Caja absoluta: la geometría es relativa al padre
    let absolute = |i: usize| -> Option<(f64, f64, f64, f64)> {
        let (mut x, mut y, w, h) = cells[i].1.geometry?;
        let mut parent = cells[i].1.parent.as_str();
        for _ in 0..20 {
            let Some(&p) = index.get(parent) else { break };
            let Some((px, py, _, _)) = cells[p].1.geometry else { break };
            x += px;
            y += py;
            parent = cells[p].1.parent.as_str();
        }
        Some((x, y, w, h))
    };
    let shapes: Vec<(usize, (f64, f64, f64, f64))> = cells
        .iter()
        .enumerate()
        .filter(|(_, (_, c))| !c.edge && !c.label.is_empty())
        .filter(|(_, (_, c))| !index.get(c.parent.as_str()).is_some_and(|&p| cells[p].1.edge))
        .filter_map(|(i, _)| absolute(i).filter(|g| g.2 > 0.0 && g.3 > 0.0).map(|g| (i, g)))
        .collect();
    let inside = |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| {
        a.0 >= b.0 && a.1 >= b.1 && a.0 + a.2 <= b.0 + b.2 && a.1 + a.3 <= b.1 + b.3 && a.2 * a.3 < b.2 * b.3
    };
    // Contenedor de cada figura: la caja con texto más chica que la contiene
    let container_of = |i: usize, g: (f64, f64, f64, f64)| -> Option<usize> {
        shapes
            .iter()
            .filter(|(j, h)| *j != i && inside(g, *h))
            .min_by(|(_, h1), (_, h2)| (h1.2 * h1.3).total_cmp(&(h2.2 * h2.3)))
            .map(|(j, _)| *j)
    };
    let mut containers: Vec<DrawioContainer> = Vec::new();
    let mut labels = Vec::new();
    let container_ids: std::collections::HashSet<usize> = shapes.iter().filter_map(|(i, g)| container_of(*i, *g)).collect();
    for (i, g) in &shapes {
        if container_ids.contains(i) {
            continue;
        }
        let label = cells[*i].1.label.clone();
        match container_of(*i, *g) {
            Some(k) => {
                let title = cells[k].1.label.clone();
                match containers.iter_mut().find(|c| c.title == title) {
                    Some(c) => c.labels.push(label),
                    None => containers.push(DrawioContainer { title, labels: vec![label] }),
                }
            }
            None => labels.push(label),
        }
    }
    // Figuras con texto sin geometría (raro): sueltas
    for (i, (_, c)) in cells.iter().enumerate() {
        if !c.edge && !c.label.is_empty() && !shapes.iter().any(|(j, _)| *j == i) && !index.get(c.parent.as_str()).is_some_and(|&p| cells[p].1.edge) {
            labels.push(c.label.clone());
        }
    }
    let edges: Vec<DrawioEdge> = cells
        .iter()
        .filter(|(_, c)| c.edge)
        .map(|(id, c)| DrawioEdge {
            from: label_of(&c.source),
            to: label_of(&c.target),
            // El texto de una conexión puede estar en una figura hija de la propia conexión
            label: if c.label.is_empty() {
                cells.iter().find(|(_, x)| x.parent == *id && !x.label.is_empty()).map(|(_, x)| x.label.clone()).unwrap_or_default()
            } else {
                c.label.clone()
            },
        })
        .filter(|e| !e.from.is_empty() || !e.to.is_empty())
        .collect();
    DrawioPage { name, containers, labels, edges }
}

#[allow(deprecated)] // `normalized_value` pide la versión de XML; aquí basta con deshacer las entidades
fn attr_of(e: &quick_xml::events::BytesStart, key: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
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
    fn estructura_con_contenedores_y_conexiones() {
        let xml = r#"<mxfile><diagram name="Flujo"><mxGraphModel><root>
            <mxCell id="0"/><mxCell id="1" parent="0"/>
            <mxCell id="g" value="ACH Operation - Receptor" vertex="1" parent="1"><mxGeometry x="0" y="0" width="400" height="300" as="geometry"/></mxCell>
            <mxCell id="a" value="Controller QTI1" vertex="1" parent="g"><mxGeometry x="10" y="10" width="100" height="40" as="geometry"/></mxCell>
            <object id="b" label="&lt;b&gt;DB ACH&lt;/b&gt;"><mxCell vertex="1" parent="1"><mxGeometry x="200" y="100" width="100" height="40" as="geometry"/></mxCell></object>
            <mxCell id="c" value="Kafka" vertex="1" parent="1"><mxGeometry x="500" y="0" width="80" height="40" as="geometry"/></mxCell>
            <mxCell id="e" value="JDBC" edge="1" parent="1" source="a" target="b"/>
            <mxCell id="f" edge="1" parent="1" source="b" target="c"/>
            <mxCell id="fl" value="publica" vertex="1" parent="f"/>
        </root></mxGraphModel></diagram></mxfile>"#;
        let pages = mxfile_structure(xml);
        assert_eq!(pages.len(), 1);
        let p = &pages[0];
        assert_eq!(p.name, "Flujo");
        assert_eq!(p.containers, vec![DrawioContainer { title: "ACH Operation - Receptor".into(), labels: vec!["Controller QTI1".into(), "DB ACH".into()] }]);
        assert_eq!(p.labels, vec!["Kafka".to_string()]);
        assert_eq!(p.edges[0], DrawioEdge { from: "Controller QTI1".into(), to: "DB ACH".into(), label: "JDBC".into() });
        assert_eq!(p.edges[1], DrawioEdge { from: "DB ACH".into(), to: "Kafka".into(), label: "publica".into() });
    }

    #[test]
    fn mxfile_dentro_de_un_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" content="&lt;mxfile&gt;&lt;diagram name=&quot;A&quot;&gt;&lt;/diagram&gt;&lt;/mxfile&gt;"><rect/></svg>"#;
        assert_eq!(mxfile_text(&svg_mxfile(svg.as_bytes()).unwrap()), "A");
        assert!(svg_mxfile(b"<svg><rect/></svg>").is_none());
    }
}
