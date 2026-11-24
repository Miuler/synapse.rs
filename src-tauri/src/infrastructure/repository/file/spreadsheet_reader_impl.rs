//! Lector de hojas de cálculo para el visor. `umya-spreadsheet` lee los formatos Open XML
//! (`.xlsx`, `.xlsm`, `.xltx`, `.xltm`) con estilos, celdas combinadas y medidas; `calamine` lee
//! el resto (`.xls`, `.xlsb`, `.ods`…) y los `.xlsx` muy grandes (es el doble de rápido), solo con
//! valores y fórmulas.
//!
//! El libro leído se guarda en una caché pequeña (se invalida si cambia el archivo); cada hoja se
//! deserializa al pedirla y las celdas se entregan por tramos de filas.
use crate::domain::spreadsheet::spreadsheet::{
    Cell, CellBlock, CellStyle, Merge, SheetInfo, SheetLayout, WorkbookInfo,
};
use crate::domain::spreadsheet::spreadsheet_reader::SpreadsheetReader;
use calamine::{Data, Range, Reader, Sheets, SheetVisible, open_workbook_auto_from_rs};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use umya_spreadsheet::structs::EnumTrait;

/// Libros que se mantienen leídos (los últimos abiertos)
const CACHE_SIZE: usize = 3;
/// Un `.xlsx` mayor que esto se lee con calamine (sin estilos): umya tardaría el doble
const LARGE_XLSX_BYTES: usize = 10 * 1024 * 1024;

type Stamp = (SystemTime, u64);

enum Book {
    Umya {
        wb: Box<umya_spreadsheet::Workbook>,
        /// Por hoja ya leída: coordenadas (fila, columna) de sus celdas, ordenadas
        index: Vec<Option<Vec<(u32, u32)>>>,
    },
    Calamine {
        reader: Box<Sheets<Cursor<Vec<u8>>>>,
        names: Vec<String>,
        loaded: Vec<Option<CalSheet>>,
    },
}

struct CalSheet {
    values: Range<Data>,
    formulas: Range<String>,
}

struct Entry {
    stamp: Stamp,
    info: WorkbookInfo,
    book: Book,
}

#[derive(Clone, Default)]
pub struct SpreadsheetReaderImpl {
    #[allow(clippy::type_complexity)]
    cache: Arc<Mutex<Vec<(PathBuf, Arc<Mutex<Entry>>)>>>,
}

impl SpreadsheetReaderImpl {
    pub fn new() -> Self {
        Self::default()
    }

    /// Libro leído (de la caché o del disco)
    fn entry(&self, vault_path: &Path, relative_path: &str) -> Result<Arc<Mutex<Entry>>, String> {
        let abs = vault_path.join(relative_path);
        let meta = std::fs::metadata(&abs).map_err(|e| format!("No se pudo leer «{relative_path}»: {e}"))?;
        let stamp = (meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), meta.len());
        // Se mantiene el candado mientras se lee: dos peticiones seguidas no leen el libro dos veces
        let mut cache = self.cache.lock().map_err(|e| e.to_string())?;
        if let Some(i) = cache.iter().position(|(p, _)| p == &abs) {
            let (p, e) = cache.remove(i);
            if e.lock().map(|e| e.stamp == stamp).unwrap_or(false) {
                cache.insert(0, (p, e.clone()));
                return Ok(e);
            }
        }
        let bytes = std::fs::read(&abs).map_err(|e| format!("No se pudo leer «{relative_path}»: {e}"))?;
        let entry = Arc::new(Mutex::new(load(relative_path, bytes, stamp)?));
        cache.insert(0, (abs, entry.clone()));
        cache.truncate(CACHE_SIZE);
        Ok(entry)
    }
}

impl SpreadsheetReader for SpreadsheetReaderImpl {
    fn open(&self, vault_path: &Path, relative_path: &str) -> Result<WorkbookInfo, String> {
        let entry = self.entry(vault_path, relative_path)?;
        let e = entry.lock().map_err(|e| e.to_string())?;
        Ok(e.info.clone())
    }

    fn layout(&self, vault_path: &Path, relative_path: &str, sheet: usize) -> Result<SheetLayout, String> {
        let entry = self.entry(vault_path, relative_path)?;
        let mut e = entry.lock().map_err(|e| e.to_string())?;
        match &mut e.book {
            Book::Umya { wb, index } => umya_layout(wb, index, sheet),
            Book::Calamine { reader, names, loaded } => cal_layout(reader, names, loaded, sheet),
        }
    }

    fn cells(
        &self,
        vault_path: &Path,
        relative_path: &str,
        sheet: usize,
        from_row: u32,
        to_row: u32,
    ) -> Result<CellBlock, String> {
        let entry = self.entry(vault_path, relative_path)?;
        let mut e = entry.lock().map_err(|e| e.to_string())?;
        match &mut e.book {
            Book::Umya { wb, index } => {
                umya_ensure(wb, index, sheet)?;
                Ok(umya_cells(wb, index[sheet].as_deref().unwrap_or(&[]), sheet, from_row, to_row))
            }
            Book::Calamine { reader, names, loaded } => {
                cal_ensure(reader, names, loaded, sheet)?;
                Ok(cal_cells(loaded[sheet].as_ref().expect("hoja cargada"), from_row, to_row))
            }
        }
    }
}

fn load(path: &str, bytes: Vec<u8>, stamp: Stamp) -> Result<Entry, String> {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let open_xml = matches!(ext.as_str(), "xlsx" | "xlsm" | "xltx" | "xltm");
    let large = open_xml && bytes.len() > LARGE_XLSX_BYTES;
    if open_xml && !large {
        // Sin deserializar las hojas: cada una se lee al pedirla
        match umya_spreadsheet::reader::xlsx::read_reader(Cursor::new(&bytes), false) {
            Ok(wb) => {
                let sheets = wb
                    .sheet_collection_no_check()
                    .iter()
                    .map(|ws| SheetInfo {
                        name: ws.name().to_string(),
                        hidden: matches!(ws.state().value_string(), "hidden" | "veryHidden"),
                    })
                    .collect::<Vec<_>>();
                let n = sheets.len();
                return Ok(Entry {
                    stamp,
                    info: WorkbookInfo { sheets, values_only: false, large: false },
                    book: Book::Umya { wb: Box::new(wb), index: (0..n).map(|_| None).collect() },
                });
            }
            Err(e) => log::warn!("umya no pudo abrir {path}, se intenta con calamine: {e}"),
        }
    }
    let reader = open_workbook_auto_from_rs(Cursor::new(bytes)).map_err(|e| format!("No se pudo abrir el libro: {e}"))?;
    let metas = reader.sheets_metadata().to_vec();
    let names: Vec<String> = metas.iter().map(|m| m.name.clone()).collect();
    let sheets = metas
        .into_iter()
        .map(|m| SheetInfo { name: m.name, hidden: !matches!(m.visible, SheetVisible::Visible) })
        .collect();
    let n = names.len();
    Ok(Entry {
        stamp,
        info: WorkbookInfo { sheets, values_only: true, large },
        book: Book::Calamine { reader: Box::new(reader), names, loaded: (0..n).map(|_| None).collect() },
    })
}

// ── umya ─────────────────────────────────────────────────────────────────────

fn umya_ensure(
    wb: &mut umya_spreadsheet::Workbook,
    index: &mut [Option<Vec<(u32, u32)>>],
    sheet: usize,
) -> Result<(), String> {
    if sheet >= index.len() {
        return Err(format!("No existe la hoja {sheet}"));
    }
    if index[sheet].is_none() {
        wb.read_sheet(sheet);
        let ws = wb.sheet(sheet).map_err(|e| e.to_string())?;
        let mut coords: Vec<(u32, u32)> =
            ws.cells().iter().map(|c| (c.coordinate().row_num(), c.coordinate().col_num())).collect();
        coords.sort_unstable();
        index[sheet] = Some(coords);
    }
    Ok(())
}

fn umya_layout(
    wb: &mut umya_spreadsheet::Workbook,
    index: &mut [Option<Vec<(u32, u32)>>],
    sheet: usize,
) -> Result<SheetLayout, String> {
    umya_ensure(wb, index, sheet)?;
    let ws = wb.sheet(sheet).map_err(|e| e.to_string())?;
    let coords = index[sheet].as_deref().unwrap_or(&[]);
    let mut out = SheetLayout {
        rows: coords.last().map(|(r, _)| *r).unwrap_or(0),
        cols: coords.iter().map(|(_, c)| *c).max().unwrap_or(0),
        cell_count: coords.len(),
        tab_color: ws.tab_color().and_then(|c| css_color(&c.argb_with_theme(wb.theme()))),
        ..Default::default()
    };
    for m in ws.merge_cells() {
        let (Some(c1), Some(r1), Some(c2), Some(r2)) =
            (m.coordinate_start_col(), m.coordinate_start_row(), m.coordinate_end_col(), m.coordinate_end_row())
        else {
            continue;
        };
        let merge = Merge { r1: r1.num(), c1: c1.num(), r2: r2.num(), c2: c2.num() };
        out.rows = out.rows.max(merge.r2);
        out.cols = out.cols.max(merge.c2);
        out.merges.push(merge);
    }
    for col in ws.column_dimensions() {
        if col.hidden() {
            out.hidden_cols.push(col.col_num());
        } else if col.width() > 0.0 {
            out.col_widths.push((col.col_num(), col.width()));
        }
    }
    for row in ws.row_dimensions() {
        if row.hidden() {
            out.hidden_rows.push(row.row_num());
        } else if row.custom_height() && row.height() > 0.0 {
            out.row_heights.push((row.row_num(), row.height()));
        }
    }
    Ok(out)
}

fn umya_cells(wb: &umya_spreadsheet::Workbook, coords: &[(u32, u32)], sheet: usize, from: u32, to: u32) -> CellBlock {
    let Ok(ws) = wb.sheet(sheet) else {
        return CellBlock::default();
    };
    let theme = wb.theme();
    let mut block = CellBlock::default();
    let mut style_ids: HashMap<CellStyle, u32> = HashMap::new();
    let start = coords.partition_point(|(r, _)| *r < from);
    for &(r, c) in coords[start..].iter().take_while(|(r, _)| *r <= to) {
        let Some(cell) = ws.cell((c, r)) else { continue };
        let formula = cell.formula();
        let f = (!formula.is_empty()).then(|| formula.trim_start_matches('=').to_string());
        let v = cell.formatted_value();
        let style = cell_style(cell.style(), theme);
        let plain = style == CellStyle::default();
        if v.is_empty() && f.is_none() && plain {
            continue;
        }
        let s = (!plain).then(|| {
            let next = style_ids.len() as u32;
            *style_ids.entry(style).or_insert(next)
        });
        block.cells.push(Cell { r, c, n: cell.data_type() == "n" && !v.is_empty(), v, f, s });
    }
    let mut styles: Vec<(CellStyle, u32)> = style_ids.into_iter().collect();
    styles.sort_by_key(|(_, i)| *i);
    block.styles = styles.into_iter().map(|(s, _)| s).collect();
    block
}

/// `FFrrggbb` / `rrggbb` → `#rrggbb`
fn css_color(argb: &str) -> Option<String> {
    let hex = match argb.len() {
        8 => &argb[2..],
        6 => argb,
        _ => return None,
    };
    hex.chars().all(|c| c.is_ascii_hexdigit()).then(|| format!("#{}", hex.to_ascii_lowercase()))
}

fn cell_style(style: &umya_spreadsheet::Style, theme: &umya_spreadsheet::structs::drawing::Theme) -> CellStyle {
    use umya_spreadsheet::structs::EnumTrait;
    let mut out = CellStyle::default();
    if let Some(font) = style.font() {
        out.bold = font.bold();
        out.italic = font.italic();
        // `underline()` da «single» aunque no haya subrayado: `val()` sí distingue
        out.underline = !matches!(font.font_underline().val().value_string(), "" | "none");
        out.strike = font.strikethrough();
        let color = font.color();
        if !color.automatic() {
            out.color = css_color(&color.argb_with_theme(theme)).filter(|c| c != "#000000");
        }
        let size = (font.size() * 10.0).round() as u32;
        if size > 0 && size != 110 {
            out.size10 = Some(size);
        }
    }
    if let Some(pf) = style.fill().and_then(|f| f.pattern_fill())
        && pf.pattern_type().value_string() != "none" {
            out.bg = pf.foreground_color().and_then(|c| css_color(&c.argb_with_theme(theme)));
        }
    if let Some(al) = style.alignment() {
        out.align = match al.horizontal().value_string() {
            "left" => Some("left"),
            "center" | "centerContinuous" => Some("center"),
            "right" => Some("right"),
            "justify" | "distributed" => Some("justify"),
            _ => None,
        }
        .map(String::from);
        out.valign = match al.vertical().value_string() {
            "top" => Some("top"),
            "center" => Some("middle"),
            _ => None,
        }
        .map(String::from);
        out.wrap = al.wrap_text();
    }
    if let Some(b) = style.borders() {
        let css = |border: &umya_spreadsheet::Border| -> Option<String> {
            let (w, kind) = match border.border_style() {
                "" | "none" => return None,
                "medium" | "mediumDashed" | "mediumDashDot" | "mediumDashDotDot" => (2, "solid"),
                "thick" => (3, "solid"),
                "double" => (3, "double"),
                "dotted" | "hair" => (1, "dotted"),
                "dashed" | "dashDot" | "dashDotDot" | "slantDashDot" => (1, "dashed"),
                _ => (1, "solid"),
            };
            let kind = if border.border_style().starts_with("medium") && border.border_style() != "medium" {
                "dashed"
            } else {
                kind
            };
            let color = border
                .color()
                .and_then(|c| css_color(&c.argb_with_theme(theme)))
                .unwrap_or_else(|| "#000000".into());
            Some(format!("{w}px {kind} {color}"))
        };
        out.border_top = css(b.top());
        out.border_right = css(b.right());
        out.border_bottom = css(b.bottom());
        out.border_left = css(b.left());
    }
    out
}

// ── calamine ─────────────────────────────────────────────────────────────────

fn cal_ensure(
    reader: &mut Sheets<Cursor<Vec<u8>>>,
    names: &[String],
    loaded: &mut [Option<CalSheet>],
    sheet: usize,
) -> Result<(), String> {
    let name = names.get(sheet).ok_or_else(|| format!("No existe la hoja {sheet}"))?;
    if loaded[sheet].is_none() {
        let values = reader.worksheet_range(name).map_err(|e| e.to_string())?;
        let formulas = reader.worksheet_formula(name).unwrap_or_default();
        loaded[sheet] = Some(CalSheet { values, formulas });
    }
    Ok(())
}

fn cal_layout(
    reader: &mut Sheets<Cursor<Vec<u8>>>,
    names: &[String],
    loaded: &mut [Option<CalSheet>],
    sheet: usize,
) -> Result<SheetLayout, String> {
    cal_ensure(reader, names, loaded, sheet)?;
    let sh = loaded[sheet].as_ref().expect("hoja cargada");
    let end = |r: &dyn Fn() -> Option<(u32, u32)>| r().map(|(a, b)| (a + 1, b + 1)).unwrap_or((0, 0));
    let (vr, vc) = end(&|| sh.values.end());
    let (fr, fc) = end(&|| sh.formulas.end());
    let mut out = SheetLayout {
        rows: vr.max(fr),
        cols: vc.max(fc),
        cell_count: sh.values.used_cells().count(),
        ..Default::default()
    };
    // Las combinadas solo las da calamine para `.xls`
    if let Sheets::Xls(x) = reader {
        for d in x.merge_cells_by_sheet_name(&names[sheet]).unwrap_or_default() {
            out.merges.push(Merge { r1: d.start.0 + 1, c1: d.start.1 + 1, r2: d.end.0 + 1, c2: d.end.1 + 1 });
        }
    }
    Ok(out)
}

fn cal_cells(sh: &CalSheet, from: u32, to: u32) -> CellBlock {
    let mut block = CellBlock::default();
    let (c0, c1) = match (sh.values.start(), sh.values.end(), sh.formulas.start(), sh.formulas.end()) {
        (Some(a), Some(b), Some(c), Some(d)) => (a.1.min(c.1), b.1.max(d.1)),
        (Some(a), Some(b), _, _) | (_, _, Some(a), Some(b)) => (a.1, b.1),
        _ => return block,
    };
    for r in from..=to {
        for c in c0..=c1 {
            let pos = (r - 1, c);
            let f = sh.formulas.get_value(pos).filter(|f| !f.is_empty()).map(|f| f.trim_start_matches('=').to_string());
            let (v, n) = match sh.values.get_value(pos) {
                None | Some(Data::Empty) => (String::new(), false),
                Some(Data::Float(x)) => (fmt_number(*x), true),
                Some(Data::Int(i)) => (i.to_string(), true),
                Some(Data::Bool(b)) => ((if *b { "VERDADERO" } else { "FALSO" }).to_string(), false),
                Some(Data::DateTime(dt)) => (fmt_excel_date(dt.as_f64()), true),
                Some(Data::Error(e)) => (format!("{e}"), false),
                Some(other) => (other.to_string(), false),
            };
            if v.is_empty() && f.is_none() {
                continue;
            }
            block.cells.push(Cell { r, c: c + 1, v, f, n, s: None });
        }
    }
    block
}

fn fmt_number(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        let s = format!("{:.10}", f);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Número de serie de Excel → `aaaa-mm-dd` (y la hora si la tiene)
fn fmt_excel_date(serial: f64) -> String {
    let days = serial.floor() as i64;
    // Excel cuenta desde 1899-12-30 (con el falso 29/02/1900 ya compensado)
    let z = days - 25569 + 719_468; // días desde 0000-03-01 (algoritmo de Howard Hinnant)
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let secs = ((serial - serial.floor()) * 86_400.0).round() as i64;
    if secs == 0 {
        format!("{y:04}-{m:02}-{d:02}")
    } else {
        format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_xlsx() -> Vec<u8> {
        let mut book = umya_spreadsheet::new_file();
        let ws = book.sheet_mut(0).unwrap();
        ws.set_name("Ventas");
        ws.cell_mut("A1").set_value("Producto");
        ws.cell_mut("A1").style_mut().font_mut().set_bold(true);
        ws.cell_mut("A2").set_value("Café");
        ws.cell_mut("B2").set_value_number(10);
        ws.cell_mut("B3").set_value_number(5);
        ws.cell_mut("B4").set_formula("SUM(B2:B3)");
        ws.cell_mut("B4").style_mut().set_background_color("FFFFFF00");
        ws.add_merge_cells("A5:B5");
        book.new_sheet("Oculta").unwrap();
        book.sheet_by_name_mut("Oculta").unwrap().set_state(umya_spreadsheet::structs::SheetStateValues::Hidden);
        book.sheet_by_name_mut("Oculta").unwrap().cell_mut("A1").set_value("Secreto");
        let mut out = Cursor::new(Vec::new());
        umya_spreadsheet::writer::xlsx::write_writer(&book, &mut out).unwrap();
        out.into_inner()
    }

    fn vault_with(name: &str, bytes: &[u8]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(name), bytes).unwrap();
        dir
    }

    #[test]
    fn lee_hojas_forma_y_celdas_por_tramos_con_umya() {
        let dir = vault_with("libro.xlsx", &sample_xlsx());
        let r = SpreadsheetReaderImpl::new();
        let info = r.open(dir.path(), "libro.xlsx").unwrap();
        assert!(!info.values_only);
        assert_eq!(info.sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Ventas", "Oculta"]);
        assert!(info.sheets[1].hidden && !info.sheets[0].hidden);

        let layout = r.layout(dir.path(), "libro.xlsx", 0).unwrap();
        assert_eq!((layout.rows, layout.cols), (5, 2));
        assert_eq!(layout.merges.len(), 1);

        let block = r.cells(dir.path(), "libro.xlsx", 0, 1, 4).unwrap();
        let cell = |rr, cc| block.cells.iter().find(|x| x.r == rr && x.c == cc).unwrap();
        assert_eq!(cell(2, 1).v, "Café");
        assert!(cell(2, 2).n);
        assert_eq!(cell(4, 2).f.as_deref(), Some("SUM(B2:B3)"));
        let a1 = &block.styles[cell(1, 1).s.unwrap() as usize];
        assert!(a1.bold && !a1.underline);
        assert_eq!(block.styles[cell(4, 2).s.unwrap() as usize].bg.as_deref(), Some("#ffff00"));
        // Solo el tramo pedido
        assert!(r.cells(dir.path(), "libro.xlsx", 0, 3, 3).unwrap().cells.iter().all(|c| c.r == 3));
        // La hoja oculta se lee al pedirla
        assert_eq!(r.cells(dir.path(), "libro.xlsx", 1, 1, 10).unwrap().cells[0].v, "Secreto");
    }

    #[test]
    fn la_cache_se_invalida_si_cambia_el_archivo() {
        let dir = vault_with("libro.xlsx", &sample_xlsx());
        let r = SpreadsheetReaderImpl::new();
        r.open(dir.path(), "libro.xlsx").unwrap();
        let mut book = umya_spreadsheet::new_file();
        book.sheet_mut(0).unwrap().set_name("Nueva");
        let mut out = Cursor::new(Vec::new());
        umya_spreadsheet::writer::xlsx::write_writer(&book, &mut out).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.path().join("libro.xlsx"), out.into_inner()).unwrap();
        assert_eq!(r.open(dir.path(), "libro.xlsx").unwrap().sheets[0].name, "Nueva");
    }

    #[test]
    fn fechas_de_excel() {
        assert_eq!(fmt_excel_date(45658.0), "2025-01-01");
        assert_eq!(fmt_excel_date(45658.5), "2025-01-01 12:00");
        assert_eq!(fmt_number(3.0), "3");
        assert_eq!(fmt_number(2.5), "2.5");
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use std::time::Instant;

    /// `SHEET_BENCH_DIR=… cargo test --release --lib spreadsheet_reader_impl::bench -- --ignored --nocapture`
    /// Genera (una vez) un libro grande en `$SHEET_BENCH_DIR` y mide abrirlo y leer tramos.
    #[test]
    #[ignore]
    fn medir_libro_grande() {
        let dir = PathBuf::from(std::env::var("SHEET_BENCH_DIR").expect("SHEET_BENCH_DIR"));
        let rows: u32 = std::env::var("SHEET_BENCH_ROWS").ok().and_then(|v| v.parse().ok()).unwrap_or(100_000);
        let cols: u32 = 50;
        let name = format!("grande-{rows}x{cols}.xlsx");
        let path = dir.join(&name);
        if !path.exists() {
            let t = Instant::now();
            let mut book = umya_spreadsheet::new_file();
            let ws = book.sheet_mut(0).unwrap();
            for c in 1..=cols {
                ws.cell_mut((c, 1)).set_value(format!("Col {c}"));
                ws.cell_mut((c, 1)).style_mut().font_mut().set_bold(true);
            }
            for r in 2..=rows {
                for c in 1..=cols {
                    if c % 10 == 0 {
                        ws.cell_mut((c, r)).set_formula(format!("SUM(A{r}:C{r})"));
                    } else if c % 3 == 0 {
                        ws.cell_mut((c, r)).set_value(format!("texto {r}-{c}"));
                    } else {
                        ws.cell_mut((c, r)).set_value_number(f64::from(r * c) / 7.0);
                    }
                }
            }
            umya_spreadsheet::writer::xlsx::write(&book, &path).unwrap();
            eprintln!("generado {} en {:?}", path.display(), t.elapsed());
        }
        eprintln!("tamaño: {:.1} MB", std::fs::metadata(&path).unwrap().len() as f64 / 1e6);
        let r = SpreadsheetReaderImpl::new();
        let t = Instant::now();
        let info = r.open(&dir, &name).unwrap();
        eprintln!("abrir: {:?} (sin estilos={})", t.elapsed(), info.values_only);
        let t = Instant::now();
        let layout = r.layout(&dir, &name, 0).unwrap();
        eprintln!("forma de la hoja: {:?} ({} celdas)", t.elapsed(), layout.cell_count);
        let t = Instant::now();
        let block = r.cells(&dir, &name, 0, 1, 200).unwrap();
        let json = serde_json::to_string(&block).unwrap();
        eprintln!("tramo de 200 filas: {:?}, {} celdas, {:.0} KB", t.elapsed(), block.cells.len(), json.len() as f64 / 1e3);
        let t = Instant::now();
        r.cells(&dir, &name, 0, rows - 200, rows).unwrap();
        eprintln!("tramo final: {:?}", t.elapsed());
        let t = Instant::now();
        r.open(&dir, &name).unwrap();
        eprintln!("reabrir (caché): {:?}", t.elapsed());
    }
}
