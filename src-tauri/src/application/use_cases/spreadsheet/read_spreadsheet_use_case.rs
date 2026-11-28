use crate::domain::spreadsheet::spreadsheet::{CellBlock, SheetLayout, SheetMatches, WorkbookInfo};
use crate::domain::spreadsheet::spreadsheet_reader::{SheetQuery, SpreadsheetReader};
use std::path::{Component, Path};

/// Tope de filas por petición de celdas
const MAX_ROWS_PER_BLOCK: u32 = 5_000;
/// Tope de celdas encontradas
const MAX_MATCHES: usize = 5_000;

/// Lee una hoja de cálculo de la bóveda para el visor (solo lectura): la lista de hojas, la forma
/// de una hoja y sus celdas por tramos de filas.
#[derive(Clone)]
pub struct ReadSpreadsheetUseCase<R: SpreadsheetReader> {
    reader: R,
}

impl<R: SpreadsheetReader> ReadSpreadsheetUseCase<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    fn check(relative_path: &str) -> Result<(), String> {
        let rel = Path::new(relative_path);
        if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(format!("Path outside the vault: {relative_path}"));
        }
        Ok(())
    }

    pub fn open(&self, vault_path: &Path, relative_path: &str) -> Result<WorkbookInfo, String> {
        Self::check(relative_path)?;
        self.reader.open(vault_path, relative_path)
    }

    pub fn layout(&self, vault_path: &Path, relative_path: &str, sheet: usize) -> Result<SheetLayout, String> {
        Self::check(relative_path)?;
        self.reader.layout(vault_path, relative_path, sheet)
    }

    pub fn cells(
        &self,
        vault_path: &Path,
        relative_path: &str,
        sheet: usize,
        from_row: u32,
        to_row: u32,
    ) -> Result<CellBlock, String> {
        Self::check(relative_path)?;
        let from = from_row.max(1);
        let to = to_row.max(from).min(from + MAX_ROWS_PER_BLOCK - 1);
        self.reader.cells(vault_path, relative_path, sheet, from, to)
    }

    pub fn find(
        &self,
        vault_path: &Path,
        relative_path: &str,
        text: String,
        hidden_sheets: bool,
        formulas: bool,
    ) -> Result<SheetMatches, String> {
        Self::check(relative_path)?;
        if text.trim().is_empty() {
            return Ok(SheetMatches::default());
        }
        let query = SheetQuery { text, hidden_sheets, formulas, limit: MAX_MATCHES };
        self.reader.find(vault_path, relative_path, &query)
    }
}
