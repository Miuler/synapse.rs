//! Libro de cálculo listo para mostrarse: lista de hojas, forma de cada hoja (combinadas, medidas,
//! ocultas) y celdas por tramos de filas con su valor formateado, su fórmula y su estilo.
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookInfo {
    pub sheets: Vec<SheetInfo>,
    /// `true` si se lee sin estilos (`.xls`/`.xlsb`/`.ods`, o un `.xlsx` muy grande)
    pub values_only: bool,
    /// Se leyó sin estilos por el tamaño del archivo
    pub large: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SheetInfo {
    pub name: String,
    pub hidden: bool,
}

/// Forma de una hoja, sin las celdas (que se piden por tramos de filas)
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SheetLayout {
    /// Última fila y columna usadas (1-based)
    pub rows: u32,
    pub cols: u32,
    pub cell_count: usize,
    pub tab_color: Option<String>,
    pub merges: Vec<Merge>,
    /// Ancho de columna en unidades de Excel (caracteres), por número de columna (1-based)
    pub col_widths: Vec<(u32, f64)>,
    /// Alto de fila en puntos, por número de fila (1-based)
    pub row_heights: Vec<(u32, f64)>,
    pub hidden_cols: Vec<u32>,
    pub hidden_rows: Vec<u32>,
}

/// Celdas de un tramo de filas; `Cell::s` indexa `styles` (propios del tramo)
#[derive(Debug, Clone, Serialize, Default)]
pub struct CellBlock {
    pub cells: Vec<Cell>,
    pub styles: Vec<CellStyle>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Cell {
    /// Fila y columna, 1-based
    pub r: u32,
    pub c: u32,
    /// Valor ya formateado (el resultado guardado si es una fórmula)
    pub v: String,
    /// Fórmula sin el `=` inicial
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f: Option<String>,
    /// Es un número (o fecha): se alinea a la derecha por defecto
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub n: bool,
    /// Índice en `CellBlock::styles`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Merge {
    pub r1: u32,
    pub c1: u32,
    pub r2: u32,
    pub c2: u32,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct CellStyle {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub strike: bool,
    /// `#rrggbb`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    /// Tamaño en puntos ×10 (entero para poder deduplicar)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size10: Option<u32>,
    /// left | center | right | justify
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// top | middle | bottom
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valign: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub wrap: bool,
    /// Bordes en CSS (`1px solid #000`): arriba, derecha, abajo, izquierda
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_top: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_right: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_bottom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_left: Option<String>,
}
