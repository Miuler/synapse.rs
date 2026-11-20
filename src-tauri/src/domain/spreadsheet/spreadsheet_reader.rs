use crate::domain::spreadsheet::spreadsheet::{CellBlock, SheetLayout, WorkbookInfo};
use std::path::Path;

/// Hojas de cálculo de la bóveda (`.xlsx`, `.xls`, `.ods`…) leídas para mostrarse. Las hojas se
/// leen al pedirlas y las celdas por tramos de filas, para no mover un libro grande de una vez.
pub trait SpreadsheetReader: Send + Sync {
    fn open(&self, vault_path: &Path, relative_path: &str) -> Result<WorkbookInfo, String>;
    fn layout(&self, vault_path: &Path, relative_path: &str, sheet: usize) -> Result<SheetLayout, String>;
    /// Celdas de las filas `from_row..=to_row` (1-based)
    fn cells(
        &self,
        vault_path: &Path,
        relative_path: &str,
        sheet: usize,
        from_row: u32,
        to_row: u32,
    ) -> Result<CellBlock, String>;
}

impl<T: SpreadsheetReader + ?Sized> SpreadsheetReader for &T {
    fn open(&self, vault_path: &Path, relative_path: &str) -> Result<WorkbookInfo, String> {
        (**self).open(vault_path, relative_path)
    }
    fn layout(&self, vault_path: &Path, relative_path: &str, sheet: usize) -> Result<SheetLayout, String> {
        (**self).layout(vault_path, relative_path, sheet)
    }
    fn cells(
        &self,
        vault_path: &Path,
        relative_path: &str,
        sheet: usize,
        from_row: u32,
        to_row: u32,
    ) -> Result<CellBlock, String> {
        (**self).cells(vault_path, relative_path, sheet, from_row, to_row)
    }
}
