//! Lee un `.docx` (Office Open XML) desde sus bytes. Se compara el texto final: lo insertado con el
//! control de cambios cuenta y lo eliminado no.
use super::xml_tree::{self, Node};
use crate::domain::docx::{
    DocxBlock, DocxCell, DocxDocument, DocxImage, DocxParser, DocxTable, DocxText,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Cursor, Read};

const MAX_FILE: usize = 200 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;
/// Tope de cada parte XML ya descomprimida.
const MAX_XML: u64 = 64 * 1024 * 1024;
/// Tope de cada imagen ya descomprimida, y de todas juntas.
const MAX_IMAGE: u64 = 64 * 1024 * 1024;
const MAX_IMAGES_TOTAL: u64 = 512 * 1024 * 1024;
/// Estilos basados en otros: tope de la cadena (puede haber ciclos).
const MAX_STYLE_CHAIN: usize = 20;

#[derive(Clone, Copy, Default)]
pub struct OoxmlParser;

type Zip<'a> = zip::ZipArchive<Cursor<&'a [u8]>>;

struct Rel {
    target: String,
    external: bool,
    kind: String,
}

#[derive(Default)]
struct Style {
    name: String,
    based_on: Option<String>,
    outline: Option<u8>,
}

/// Campo de Word en curso (`fldChar`): puede abarcar varios párrafos.
#[derive(Default)]
struct Field {
    instruction: String,
    in_result: bool,
}

struct Walker<'a, 'b> {
    zip: &'a mut Zip<'b>,
    /// Carpeta de la parte principal (`word`).
    base: String,
    rels: HashMap<String, Rel>,
    styles: HashMap<String, Style>,
    /// Huella por ruta dentro del zip (la misma imagen puede salir varias veces).
    hashes: HashMap<String, String>,
    images_read: u64,
    fields: Vec<Field>,
}

/// Lo reunido de un párrafo.
#[derive(Default)]
struct Paragraph {
    text: String,
    images: Vec<DocxImage>,
    /// Bloques de sus cuadros de texto.
    boxes: Vec<DocxBlock>,
}

fn open(bytes: &[u8]) -> Result<Zip<'_>, String> {
    if bytes.len() > MAX_FILE {
        return Err("El documento es demasiado grande para compararlo".into());
    }
    let zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("No es un documento de Word válido: {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err("El documento tiene demasiadas partes".into());
    }
    Ok(zip)
}

/// Contenido de una parte, o `None` si no está. Nunca descomprime más de `limit`.
fn read_part(zip: &mut Zip, name: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
    let file = match zip.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(format!("No se pudo leer {name}: {e}")),
    };
    let mut out = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut out)
        .map_err(|e| format!("No se pudo leer {name}: {e}"))?;
    if out.len() as u64 > limit {
        return Err(format!("{name} es demasiado grande para compararlo"));
    }
    Ok(Some(out))
}

fn read_xml(zip: &mut Zip, name: &str) -> Result<Option<Node>, String> {
    let Some(bytes) = read_part(zip, name, MAX_XML)? else {
        return Ok(None);
    };
    let text = encoding_rs::Encoding::for_bom(&bytes)
        .unwrap_or((encoding_rs::UTF_8, 0))
        .0
        .decode(&bytes)
        .0
        .into_owned();
    xml_tree::parse(&text).map(Some)
}

/// Une `target` a la carpeta `base`, sin salir de la raíz del zip.
fn resolve(base: &str, target: &str) -> String {
    let joined = match target.strip_prefix('/') {
        Some(abs) => abs.to_string(),
        None => format!("{base}/{target}"),
    };
    let mut parts: Vec<&str> = Vec::new();
    for p in joined.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(p),
        }
    }
    parts.join("/")
}

fn read_rels(zip: &mut Zip, name: &str) -> Result<HashMap<String, Rel>, String> {
    let Some(root) = read_xml(zip, name)? else {
        return Ok(HashMap::new());
    };
    Ok(root
        .children
        .iter()
        .filter_map(|r| {
            let rel = Rel {
                target: r.attr("Target")?.to_string(),
                external: r.attr("TargetMode") == Some("External"),
                kind: r.attr("Type").unwrap_or_default().to_string(),
            };
            Some((r.attr("Id")?.to_string(), rel))
        })
        .collect())
}

/// Parte principal del documento, su carpeta y sus relaciones.
fn main_part(zip: &mut Zip) -> Result<(String, String, HashMap<String, Rel>), String> {
    let part = read_rels(zip, "_rels/.rels")?
        .values()
        .find(|r| !r.external && r.kind.ends_with("/officeDocument"))
        .map(|r| resolve("", &r.target))
        .unwrap_or_else(|| "word/document.xml".to_string());
    let (dir, file) = part.rsplit_once('/').unwrap_or(("", &part));
    let rels = read_rels(zip, &resolve("", &format!("{dir}/_rels/{file}.rels")))?;
    Ok((part.clone(), dir.to_string(), rels))
}

fn read_styles(
    zip: &mut Zip,
    base: &str,
    rels: &HashMap<String, Rel>,
) -> Result<HashMap<String, Style>, String> {
    let part = rels
        .values()
        .find(|r| !r.external && r.kind.ends_with("/styles"))
        .map(|r| resolve(base, &r.target))
        .unwrap_or_else(|| resolve(base, "styles.xml"));
    let Some(root) = read_xml(zip, &part)? else {
        return Ok(HashMap::new());
    };
    Ok(root
        .children
        .iter()
        .filter(|s| s.local() == "style")
        .filter_map(|s| {
            let val = |n: &str| s.child(n).and_then(|c| c.attr("val"));
            let style = Style {
                name: val("name").unwrap_or_default().to_lowercase(),
                based_on: val("basedOn").map(str::to_string),
                outline: s.child("pPr").and_then(outline_level),
            };
            Some((s.attr("styleId")?.to_string(), style))
        })
        .collect())
}

fn outline_level(ppr: &Node) -> Option<u8> {
    ppr.child("outlineLvl")?.attr("val")?.parse().ok()
}

/// «heading 2» → 2. Word guarda en inglés el nombre de los estilos propios en cualquier idioma.
fn heading_name(name: &str) -> Option<u8> {
    if name == "title" {
        return Some(1);
    }
    let n: u8 = name.strip_prefix("heading ")?.trim().parse().ok()?;
    (1..=9).contains(&n).then_some(n)
}

fn mime_of(path: &str) -> Option<&'static str> {
    Some(match path.rsplit('.').next()?.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" | "jpe" | "jfif" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => return None,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Campos cuyo resultado cambia solo (índices, números de página, fechas): no se comparan.
fn is_volatile(instruction: &str) -> bool {
    matches!(
        instruction
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_uppercase()
            .as_str(),
        "TOC" | "PAGE" | "NUMPAGES" | "SECTIONPAGES" | "PAGEREF" | "DATE" | "TIME" | "SAVEDATE" | "PRINTDATE"
    )
}

/// De `mc:AlternateContent` se lee una sola rama: las dos traen el mismo contenido.
fn alternate(node: &Node) -> Option<&Node> {
    node.child("Choice").or_else(|| node.child("Fallback"))
}

impl Walker<'_, '_> {
    fn blocks(&mut self, node: &Node, out: &mut Vec<DocxBlock>) -> Result<(), String> {
        for c in &node.children {
            match c.local() {
                "p" => self.paragraph(c, out)?,
                "tbl" => {
                    let table = self.table(c)?;
                    if !table.rows.is_empty() {
                        out.push(DocxBlock::Table(table));
                    }
                }
                "del" | "moveFrom" | "sectPr" | "sdtPr" => {}
                "AlternateContent" => {
                    if let Some(a) = alternate(c) {
                        self.blocks(a, out)?;
                    }
                }
                // Controles de contenido, inserciones, XML propio…
                _ => self.blocks(c, out)?,
            }
        }
        Ok(())
    }

    fn paragraph(&mut self, node: &Node, out: &mut Vec<DocxBlock>) -> Result<(), String> {
        let mut p = Paragraph::default();
        self.inline(node, &mut p)?;
        let text = p.text.trim();
        if !text.is_empty() {
            let ppr = node.child("pPr");
            let list_level = ppr.and_then(|p| p.child("numPr")).and_then(|n| {
                let val = |name: &str| n.child(name).and_then(|c| c.attr("val"));
                if val("numId") == Some("0") {
                    return None;
                }
                Some(val("ilvl").and_then(|v| v.parse().ok()).unwrap_or(0))
            });
            out.push(DocxBlock::Text(DocxText {
                text: text.to_string(),
                heading: self.heading(ppr),
                list_level,
            }));
        }
        out.extend(p.images.into_iter().map(DocxBlock::Image));
        out.extend(p.boxes);
        Ok(())
    }

    fn heading(&self, ppr: Option<&Node>) -> Option<u8> {
        let ppr = ppr?;
        // 9 es «texto independiente»
        let level = |l: u8| (l < 9).then_some(l + 1);
        if let Some(l) = outline_level(ppr) {
            return level(l);
        }
        let mut id = ppr.child("pStyle")?.attr("val")?.to_string();
        for _ in 0..MAX_STYLE_CHAIN {
            let Some(style) = self.styles.get(&id) else {
                // Sin styles.xml solo queda el id en inglés
                return heading_name(&id.to_lowercase().replace("heading", "heading "));
            };
            if let Some(n) = heading_name(&style.name) {
                return Some(n);
            }
            if let Some(l) = style.outline {
                return level(l);
            }
            id = style.based_on.clone()?;
        }
        None
    }

    /// ¿Se está dentro del resultado de un campo que cambia solo?
    fn in_volatile_field(&self) -> bool {
        self.fields
            .iter()
            .any(|f| f.in_result && is_volatile(&f.instruction))
    }

    fn push(&self, p: &mut Paragraph, s: &str) {
        if !self.in_volatile_field() {
            p.text.push_str(s);
        }
    }

    fn inline(&mut self, node: &Node, p: &mut Paragraph) -> Result<(), String> {
        for c in &node.children {
            match c.local() {
                "t" => self.push(p, &c.text),
                "tab" | "ptab" => self.push(p, "\t"),
                "br" | "cr" => {
                    // Los saltos de página y de columna no son texto
                    if !matches!(c.attr("type"), Some("page" | "column")) {
                        self.push(p, "\n");
                    }
                }
                "noBreakHyphen" => self.push(p, "-"),
                "sym" => {
                    let symbol = c
                        .attr("char")
                        .and_then(|h| u32::from_str_radix(h, 16).ok())
                        .and_then(char::from_u32);
                    if let Some(s) = symbol {
                        self.push(p, s.encode_utf8(&mut [0; 4]));
                    }
                }
                "pPr" | "rPr" | "sdtPr" | "del" | "moveFrom" | "delText" => {}
                "r" if c.child("rPr").is_some_and(|r| r.flag("vanish")) => self.fields_only(c),
                "fldChar" => self.field_char(c),
                "instrText" => self.instruction(c),
                "fldSimple" => {
                    if !c.attr("instr").is_some_and(is_volatile) {
                        self.inline(c, p)?;
                    }
                }
                "AlternateContent" => {
                    if let Some(a) = alternate(c) {
                        self.inline(a, p)?;
                    }
                }
                "txbxContent" => {
                    let mut boxes = Vec::new();
                    self.blocks(c, &mut boxes)?;
                    p.boxes.append(&mut boxes);
                }
                "drawing" | "pict" | "object" => {
                    let alt = alt_text(c);
                    self.images(c, &alt, p)?;
                }
                _ => self.inline(c, p)?,
            }
        }
        Ok(())
    }

    /// De un run oculto solo importan los campos que abre o cierra.
    fn fields_only(&mut self, run: &Node) {
        for c in &run.children {
            match c.local() {
                "fldChar" => self.field_char(c),
                "instrText" => self.instruction(c),
                _ => {}
            }
        }
    }

    fn field_char(&mut self, c: &Node) {
        match c.attr("fldCharType") {
            Some("begin") => self.fields.push(Field::default()),
            Some("separate") => {
                if let Some(f) = self.fields.last_mut() {
                    f.in_result = true;
                }
            }
            Some("end") => {
                self.fields.pop();
            }
            _ => {}
        }
    }

    fn instruction(&mut self, c: &Node) {
        if let Some(f) = self.fields.last_mut() {
            f.instruction.push_str(&c.text);
        }
    }

    /// Imágenes (y cuadros de texto) de un dibujo, que puede agrupar varias.
    fn images(&mut self, node: &Node, alt: &str, p: &mut Paragraph) -> Result<(), String> {
        for c in &node.children {
            match c.local() {
                "blip" => {
                    if let Some(id) = c.attr("embed").or_else(|| c.attr("link")) {
                        let image = self.image(id, alt)?;
                        p.images.extend(image);
                    }
                }
                "imagedata" => {
                    if let Some(id) = c.prefixed_attr("id") {
                        let title = c.attr("title").unwrap_or(alt).to_string();
                        let image = self.image(id, &title)?;
                        p.images.extend(image);
                    }
                }
                "txbxContent" => {
                    let mut boxes = Vec::new();
                    self.blocks(c, &mut boxes)?;
                    p.boxes.append(&mut boxes);
                }
                "AlternateContent" => {
                    if let Some(a) = alternate(c) {
                        self.images(a, alt, p)?;
                    }
                }
                _ => self.images(c, alt, p)?,
            }
        }
        Ok(())
    }

    fn image(&mut self, rel_id: &str, alt: &str) -> Result<Option<DocxImage>, String> {
        let Some(rel) = self.rels.get(rel_id) else {
            return Ok(None);
        };
        let name = |path: &str| path.rsplit('/').next().unwrap_or(path).to_string();
        if rel.external {
            return Ok(Some(DocxImage {
                filename: rel.target.clone(),
                alt: alt.to_string(),
                external: true,
                ..DocxImage::default()
            }));
        }
        let path = resolve(&self.base, &rel.target);
        let sha256 = match self.hashes.get(&path) {
            Some(h) => h.clone(),
            None => {
                let hash = match read_part(self.zip, &path, MAX_IMAGE)? {
                    Some(bytes) => {
                        self.images_read += bytes.len() as u64;
                        if self.images_read > MAX_IMAGES_TOTAL {
                            return Err("Las imágenes del documento son demasiado grandes para compararlas".into());
                        }
                        hex(&Sha256::digest(&bytes))
                    }
                    None => String::new(),
                };
                self.hashes.insert(path.clone(), hash.clone());
                hash
            }
        };
        Ok(Some(DocxImage {
            sha256,
            filename: name(&path),
            mime: mime_of(&path).map(str::to_string),
            alt: alt.to_string(),
            external: false,
        }))
    }

    fn table(&mut self, node: &Node) -> Result<DocxTable, String> {
        let mut rows = Vec::new();
        for tr in named(node, "tr") {
            // Fila eliminada con el control de cambios
            if tr.child("trPr").is_some_and(|p| p.child("del").is_some()) {
                continue;
            }
            let mut cells = Vec::new();
            for tc in named(tr, "tc") {
                let pr = tc.child("tcPr");
                let mut cell = DocxCell {
                    col_span: pr
                        .and_then(|p| p.child("gridSpan"))
                        .and_then(|g| g.attr("val"))
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(1)
                        .clamp(1, 1_000),
                    merged: pr
                        .and_then(|p| p.child("vMerge"))
                        .is_some_and(|v| v.attr("val") != Some("restart")),
                    ..DocxCell::default()
                };
                self.blocks(tc, &mut cell.blocks)?;
                cells.push(cell);
            }
            rows.push(cells);
        }
        Ok(DocxTable { rows })
    }
}

fn alt_text(drawing: &Node) -> String {
    for c in &drawing.children {
        if c.local() == "docPr" {
            return c.attr("descr").or(c.attr("title")).unwrap_or_default().to_string();
        }
        let found = alt_text(c);
        if !found.is_empty() {
            return found;
        }
    }
    String::new()
}

/// Filas o celdas de una tabla, también dentro de controles de contenido (no las de tablas anidadas).
fn named<'n>(node: &'n Node, name: &str) -> Vec<&'n Node> {
    let mut out = Vec::new();
    for c in &node.children {
        match c.local() {
            l if l == name => out.push(c),
            "sdt" | "sdtContent" | "ins" | "moveTo" | "customXml" => out.extend(named(c, name)),
            _ => {}
        }
    }
    out
}

impl DocxParser for OoxmlParser {
    fn parse(&self, bytes: &[u8]) -> Result<DocxDocument, String> {
        let mut zip = open(bytes)?;
        let (part, base, rels) = main_part(&mut zip)?;
        let styles = read_styles(&mut zip, &base, &rels)?;
        let root = read_xml(&mut zip, &part)?
            .ok_or_else(|| "No es un documento de Word: falta su contenido".to_string())?;
        let body = root
            .child("body")
            .ok_or_else(|| "No es un documento de Word: falta su contenido".to_string())?;
        let mut walker = Walker {
            zip: &mut zip,
            base,
            rels,
            styles,
            hashes: HashMap::new(),
            images_read: 0,
            fields: Vec::new(),
        };
        let mut blocks = Vec::new();
        walker.blocks(body, &mut blocks)?;
        Ok(DocxDocument { blocks })
    }

    fn image(&self, bytes: &[u8], sha256: &str) -> Result<Option<(String, Vec<u8>)>, String> {
        let mut zip = open(bytes)?;
        let (_, base, rels) = main_part(&mut zip)?;
        let mut read = 0u64;
        for rel in rels.values().filter(|r| !r.external) {
            let path = resolve(&base, &rel.target);
            let Some(mime) = mime_of(&path) else { continue };
            let Some(image) = read_part(&mut zip, &path, MAX_IMAGE)? else {
                continue;
            };
            read += image.len() as u64;
            if read > MAX_IMAGES_TOTAL {
                break;
            }
            if hex(&Sha256::digest(&image)) == sha256 {
                return Ok(Some((mime.to_string(), image)));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing""#;

    pub fn zip_of(parts: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        for (name, data) in parts {
            zip.start_file(*name, SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
        buffer
    }

    pub fn document(body: &str) -> String {
        format!(r#"<?xml version="1.0" encoding="UTF-8"?><w:document {NS}><w:body>{body}</w:body></w:document>"#)
    }

    pub fn docx(body: &str) -> Vec<u8> {
        zip_of(&[("word/document.xml", document(body).as_bytes())])
    }

    pub fn blocks(body: &str) -> Vec<DocxBlock> {
        OoxmlParser.parse(&docx(body)).unwrap().blocks
    }

    /// Documento con una imagen, con el `rId` y el nombre indicados.
    pub fn with_image(rel_id: &str, name: &str, data: &[u8]) -> Vec<u8> {
        let rels = format!(r#"<Relationships><Relationship Id="{rel_id}" Type="http://x/image" Target="media/{name}"/></Relationships>"#);
        let body = format!(r#"<w:p><w:r><w:t>Antes</w:t><w:drawing><wp:inline><wp:docPr id="1" name="x" descr="un gato"/><a:graphic><a:blip r:embed="{rel_id}"/></a:graphic></wp:inline></w:drawing></w:r></w:p>"#);
        zip_of(&[
            ("word/document.xml", document(&body).as_bytes()),
            ("word/_rels/document.xml.rels", rels.as_bytes()),
            (&format!("word/media/{name}"), data),
        ])
    }

    fn texts(blocks: &[DocxBlock]) -> Vec<String> {
        blocks.iter().map(DocxBlock::plain_text).collect()
    }

    #[test]
    fn une_los_runs_e_ignora_los_rsid() {
        let a = blocks(r#"<w:p w:rsidR="00A1"><w:r w:rsidR="01"><w:t>Hola mundo</w:t></w:r></w:p>"#);
        let b = blocks(r#"<w:p w:rsidR="00FF"><w:r><w:t>Ho</w:t></w:r><w:proofErr w:type="spellStart"/><w:r w:rsidRPr="9"><w:rPr><w:b/></w:rPr><w:t xml:space="preserve">la </w:t></w:r><w:r><w:t>mundo</w:t></w:r></w:p>"#);
        assert_eq!(a, b);
        assert_eq!(texts(&a), ["Hola mundo"]);
    }

    #[test]
    fn conserva_entidades_tabuladores_y_saltos() {
        let b = blocks(r#"<w:p><w:hyperlink r:id="x"><w:r><w:t>I&amp;D a&#241;o</w:t></w:r></w:hyperlink><w:r><w:tab/><w:t>b</w:t><w:br/><w:t>c</w:t><w:noBreakHyphen/><w:t>d</w:t><w:br w:type="page"/></w:r></w:p>"#);
        assert_eq!(texts(&b), ["I&D año\tb\nc-d"]);
    }

    #[test]
    fn reconoce_titulos_en_cualquier_idioma() {
        let styles = r#"<w:styles xmlns:w="w">
            <w:style w:styleId="Ttulo2"><w:name w:val="heading 2"/></w:style>
            <w:style w:styleId="MiTitulo"><w:name w:val="Mi título"/><w:basedOn w:val="Ttulo2"/></w:style>
            <w:style w:styleId="Anexo"><w:name w:val="Anexo"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>
            <w:style w:styleId="A"><w:name w:val="a"/><w:basedOn w:val="B"/></w:style>
            <w:style w:styleId="B"><w:name w:val="b"/><w:basedOn w:val="A"/></w:style></w:styles>"#;
        let p = |s: &str| format!(r#"<w:p><w:pPr><w:pStyle w:val="{s}"/></w:pPr><w:r><w:t>{s}</w:t></w:r></w:p>"#);
        let body: String = ["Ttulo2", "MiTitulo", "Anexo", "A", "Heading3"].iter().map(|s| p(s)).collect();
        let bytes = zip_of(&[("word/document.xml", document(&body).as_bytes()), ("word/styles.xml", styles.as_bytes())]);
        let levels: Vec<_> = OoxmlParser.parse(&bytes).unwrap().blocks.iter().map(|b| match b {
            DocxBlock::Text(t) => t.heading,
            _ => None,
        }).collect();
        assert_eq!(levels, [Some(2), Some(2), Some(1), None, Some(3)]);
    }

    #[test]
    fn el_control_de_cambios_da_el_texto_final() {
        let b = blocks(r#"<w:p><w:r><w:t xml:space="preserve">uno </w:t></w:r><w:del><w:r><w:delText>viejo</w:delText></w:r></w:del><w:ins><w:r><w:t>nuevo</w:t></w:r></w:ins></w:p><w:del><w:p><w:r><w:t>x</w:t></w:r></w:p></w:del>"#);
        assert_eq!(texts(&b), ["uno nuevo"]);
    }

    #[test]
    fn alternate_content_se_lee_una_vez() {
        let caja = |t: &str| format!("<w:txbxContent><w:p><w:r><w:t>{t}</w:t></w:r></w:p></w:txbxContent>");
        let b = blocks(&format!(r#"<w:p><w:r><w:t>cuerpo</w:t><mc:AlternateContent><mc:Choice><w:drawing>{}</w:drawing></mc:Choice><mc:Fallback><w:pict>{}</w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p>"#, caja("caja"), caja("caja")));
        assert_eq!(texts(&b), ["cuerpo", "caja"]);
    }

    #[test]
    fn los_campos_que_cambian_solos_no_cuentan() {
        let campo = |instr: &str, res: &str| format!(r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> {instr} </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>{res}</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r>"#);
        let b = blocks(&format!(r#"<w:p><w:r><w:t xml:space="preserve">Página </w:t></w:r>{}<w:r><w:t xml:space="preserve"> ver </w:t></w:r>{}<w:fldSimple w:instr="NUMPAGES"><w:r><w:t>9</w:t></w:r></w:fldSimple></w:p>"#, campo("PAGE", "3"), campo("REF x", "Anexo")));
        assert_eq!(texts(&b), ["Página  ver Anexo"]);
    }

    #[test]
    fn tablas_con_celdas_combinadas_y_anidadas() {
        let c = |pr: &str, t: &str| format!("<w:tc><w:tcPr>{pr}</w:tcPr><w:p><w:r><w:t>{t}</w:t></w:r></w:p></w:tc>");
        let nested = "<w:tc><w:tbl><w:tr><w:tc><w:p><w:r><w:t>dentro</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p/></w:tc>";
        let b = blocks(&format!(
            "<w:tbl><w:tr>{}{}</w:tr><w:sdt><w:sdtContent><w:tr>{}{}</w:tr></w:sdtContent></w:sdt><w:tr><w:trPr><w:del/></w:trPr>{}</w:tr></w:tbl>",
            c(r#"<w:gridSpan w:val="2"/>"#, "ancha"), c(r#"<w:vMerge w:val="restart"/>"#, "alta"), nested, c("<w:vMerge/>", ""), c("", "borrada"),
        ));
        let DocxBlock::Table(t) = &b[0] else { panic!() };
        assert_eq!(t.rows.len(), 2);
        assert_eq!((t.rows[0][0].col_span, t.rows[0][1].merged, t.rows[1][1].merged), (2, false, true));
        assert!(matches!(&t.rows[1][0].blocks[0], DocxBlock::Table(_)));
    }

    #[test]
    fn la_imagen_se_identifica_por_su_contenido() {
        let img = |bytes: &[u8]| OoxmlParser.parse(bytes).unwrap().blocks[1].clone();
        let a = img(&with_image("rId4", "image1.png", b"gato"));
        let b = img(&with_image("rId99", "image7.png", b"gato"));
        let c = img(&with_image("rId4", "image1.png", b"perro"));
        assert_eq!(a.key(), b.key());
        assert_ne!(a.key(), c.key());
        let DocxBlock::Image(i) = &a else { panic!() };
        assert_eq!((i.alt.as_str(), i.mime.as_deref()), ("un gato", Some("image/png")));
        let found = OoxmlParser.image(&with_image("r", "x.png", b"gato"), &i.sha256).unwrap();
        assert_eq!(found, Some(("image/png".to_string(), b"gato".to_vec())));
        // EMF: cuenta como imagen, pero no se puede mostrar
        let DocxBlock::Image(emf) = img(&with_image("r", "x.emf", b"gato")) else { panic!() };
        assert_eq!(emf.mime, None);
        assert_eq!(OoxmlParser.image(&with_image("r", "x.emf", b"gato"), &emf.sha256).unwrap(), None);
    }

    #[test]
    fn las_imagenes_externas_no_se_leen() {
        let rels = r#"<Relationships><Relationship Id="r1" Type="http://x/image" Target="https://espia.example/x.png" TargetMode="External"/></Relationships>"#;
        let body = r#"<w:p><w:r><w:drawing><a:blip r:link="r1"/></w:drawing></w:r></w:p>"#;
        let bytes = zip_of(&[("word/document.xml", document(body).as_bytes()), ("word/_rels/document.xml.rels", rels.as_bytes())]);
        let DocxBlock::Image(i) = OoxmlParser.parse(&bytes).unwrap().blocks[0].clone() else { panic!() };
        assert!(i.external && i.sha256.is_empty());
    }

    #[test]
    fn los_documentos_danados_dan_error_sin_panico() {
        for bytes in [b"".to_vec(), b"no es un zip".to_vec(), zip_of(&[("otro.xml", b"<a/>")]), docx("<w:p>"), docx("</w:p>")] {
            assert!(OoxmlParser.parse(&bytes).is_err());
        }
        let full = docx("<w:p><w:r><w:t>hola</w:t></w:r></w:p>");
        for n in (0..full.len()).step_by(7) {
            let _ = OoxmlParser.parse(&full[..n]);
        }
        let deep = format!("{}{}", "<w:sdt>".repeat(5_000), "</w:sdt>".repeat(5_000));
        assert!(OoxmlParser.parse(&docx(&deep)).unwrap_err().contains("anidados"));
    }

    #[test]
    fn una_bomba_zip_no_se_descomprime_entera() {
        let big = vec![b' '; (MAX_XML + 2) as usize];
        let bomb = zip_of(&[("word/document.xml", &big)]);
        assert!(bomb.len() < 1024 * 1024);
        assert!(OoxmlParser.parse(&bomb).unwrap_err().contains("demasiado grande"));
    }

    #[test]
    fn lee_el_documento_de_ejemplo() {
        let doc = OoxmlParser.parse(include_bytes!("../../../../e2e/fixtures/informe.docx")).unwrap();
        // Escrito con LibreOffice: el «título» es un párrafo en negrita, no un estilo de título
        assert_eq!(texts(&doc.blocks)[..2], ["Informe trimestral", "Texto normal del documento."]);
        assert!(doc.blocks.iter().any(|b| matches!(b, DocxBlock::Table(t) if t.rows.len() > 1)));
    }

    #[test]
    fn las_rutas_no_salen_del_zip() {
        assert_eq!(resolve("word", "../../../etc/passwd"), "etc/passwd");
        assert_eq!(resolve("word", "/word/media/a.png"), "word/media/a.png");
        assert_eq!(resolve("word", "media/a.png"), "word/media/a.png");
    }
}
