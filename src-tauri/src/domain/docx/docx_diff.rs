//! Diferencias entre dos versiones de un documento de Word.
use super::docx_document::{DocxBlock, DocxImage};
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxDiff {
    pub old_exists: bool,
    pub new_exists: bool,
    /// Los dos archivos son idénticos byte a byte (si no hay cambios y esto es falso, difieren en
    /// algo que no se compara, como el formato).
    pub same_bytes: bool,
    /// Se agotó el tiempo: el resultado es correcto, pero puede no ser el más corto.
    pub approximate: bool,
    pub entries: Vec<DocxDiffEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DocxDiffEntry {
    Equal { block: DocxBlock },
    Added { block: DocxBlock },
    Removed { block: DocxBlock },
    Modified { change: DocxBlockChange },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DocxBlockChange {
    #[serde(rename_all = "camelCase")]
    Text {
        old_heading: Option<u8>,
        new_heading: Option<u8>,
        list_level: Option<u8>,
        segments: Vec<DocxSegment>,
    },
    Table { rows: Vec<DocxRowChange> },
    Image { old: DocxImage, new: DocxImage },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DocxChangeKind {
    Equal,
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocxSegment {
    /// `Equal`, `Added` o `Removed`.
    pub kind: DocxChangeKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocxRowChange {
    pub kind: DocxChangeKind,
    pub cells: Vec<DocxCellChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocxCellChange {
    pub kind: DocxChangeKind,
    pub col_span: u32,
    pub merged: bool,
    pub entries: Vec<DocxDiffEntry>,
}
