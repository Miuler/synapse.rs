//! Documento de Word reducido a lo que se compara: títulos, párrafos, tablas e imágenes del cuerpo.
//! El formato, los encabezados, pies, notas y comentarios quedan fuera.
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocxDocument {
    pub blocks: Vec<DocxBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DocxBlock {
    #[serde(rename_all = "camelCase")]
    Text(DocxText),
    Table(DocxTable),
    Image(DocxImage),
}

/// Párrafo o título.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxText {
    pub text: String,
    /// Nivel de título (1-9), o `None` si es un párrafo.
    pub heading: Option<u8>,
    /// Nivel de lista (0 = primero), o `None` si no está en una lista.
    pub list_level: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DocxTable {
    pub rows: Vec<Vec<DocxCell>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxCell {
    pub blocks: Vec<DocxBlock>,
    pub col_span: u32,
    /// Continúa la celda de arriba (combinada en vertical).
    pub merged: bool,
}

/// Imagen, sin sus bytes: se piden aparte por su huella. El `rId` no se guarda: Word lo cambia al guardar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxImage {
    /// SHA-256 del contenido; vacío si está fuera del documento o no se pudo leer.
    pub sha256: String,
    pub filename: String,
    /// `None` si el formato no se puede mostrar (EMF, WMF…).
    pub mime: Option<String>,
    pub alt: String,
    /// Enlazada, fuera del documento: no se descarga.
    pub external: bool,
}

impl DocxBlock {
    /// Lo que identifica al bloque al comparar. Ni el nivel de lista ni el nombre de la imagen cuentan.
    pub fn key(&self) -> String {
        match self {
            DocxBlock::Text(t) => format!("T{}|{}", t.heading.unwrap_or(0), t.text),
            DocxBlock::Table(t) => {
                let mut out = String::from("B[");
                for row in &t.rows {
                    out.push_str(&row_key(row));
                    out.push('\u{1e}');
                }
                out.push(']');
                out
            }
            DocxBlock::Image(i) if i.sha256.is_empty() => format!("I{}|{}", i.external, i.filename),
            DocxBlock::Image(i) => format!("I{}", i.sha256),
        }
    }

    /// Todo su texto, para medir parecidos.
    pub fn plain_text(&self) -> String {
        match self {
            DocxBlock::Text(t) => t.text.clone(),
            DocxBlock::Image(i) => i.alt.clone(),
            DocxBlock::Table(t) => {
                let cells = t.rows.iter().flatten().map(DocxCell::plain_text);
                cells.collect::<Vec<_>>().join(" ")
            }
        }
    }
}

impl DocxCell {
    pub fn key(&self) -> String {
        let mut out = format!("{}{}:", self.col_span, if self.merged { "m" } else { "" });
        for b in &self.blocks {
            out.push_str(&b.key());
            out.push('\u{1f}');
        }
        out
    }

    pub fn plain_text(&self) -> String {
        let blocks = self.blocks.iter().map(DocxBlock::plain_text);
        blocks.collect::<Vec<_>>().join(" ")
    }
}

pub fn row_key(row: &[DocxCell]) -> String {
    let cells = row.iter().map(DocxCell::key);
    cells.collect::<Vec<_>>().join("\u{1d}")
}
