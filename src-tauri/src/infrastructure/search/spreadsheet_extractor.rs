//! Texto de una planilla (Excel `.xlsx`/`.xls`/`.xlsm`, OpenDocument `.ods`) para el índice
//! full-text: el nombre de cada hoja y sus filas, con las celdas separadas por ` | `.

use calamine::{Data, Reader, open_workbook_auto_from_rs};
use std::io::Cursor;

pub fn spreadsheet_text(bytes: &[u8], max_chars: usize) -> Result<String, String> {
    spreadsheet_extract(bytes, max_chars).map(|e| e.text)
}

/// Texto de una planilla para el índice y, línea a línea, dónde está cada fila.
pub struct SpreadsheetText {
    /// `Hoja: X` y una línea por fila con celdas, separadas por ` | `.
    pub text: String,
    /// Una línea por línea de `text`: vacía para `Hoja:`, y `r37 A,C,D` (número de fila y letra
    /// de la columna de cada celda, en el mismo orden) para una fila.
    pub layout: String,
    /// Se cortó por el límite de caracteres.
    pub truncated: bool,
}

pub fn spreadsheet_extract(bytes: &[u8], max_chars: usize) -> Result<SpreadsheetText, String> {
    let mut book = open_workbook_auto_from_rs(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut out = SpreadsheetText {
        text: String::new(),
        layout: String::new(),
        truncated: false,
    };
    for name in book.sheet_names().to_vec() {
        let Ok(range) = book.worksheet_range(&name) else {
            continue;
        };
        out.text.push_str(&format!("Hoja: {name}\n"));
        out.layout.push('\n');
        let (first_row, first_col) = range.start().unwrap_or((0, 0));
        for (r, row) in range.rows().enumerate() {
            let mut columns = Vec::new();
            let cells: Vec<String> = row
                .iter()
                .enumerate()
                .filter(|(_, c)| !matches!(c, Data::Empty))
                .map(|(j, c)| (j, c.to_string().trim().to_string()))
                .filter(|(_, c)| !c.is_empty())
                .map(|(j, c)| {
                    columns.push(column_letters(first_col as usize + j));
                    c
                })
                .collect();
            if cells.is_empty() {
                continue;
            }
            out.text.push_str(&cells.join(" | "));
            out.text.push('\n');
            out.layout.push_str(&format!(
                "r{} {}\n",
                first_row as usize + r + 1,
                columns.join(",")
            ));
            if out.text.len() >= max_chars {
                out.truncated = true;
                return Ok(out);
            }
        }
    }
    Ok(out)
}

/// Letra de una columna a partir de su índice desde 0 (0 → A, 25 → Z, 26 → AA).
pub fn column_letters(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push((b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    letters.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// `.xlsx` mínimo con una hoja «Montos» y celdas de texto en línea y numéricas.
    fn xlsx() -> Vec<u8> {
        let files = [
            (
                "[Content_Types].xml",
                r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
            ),
            (
                "_rels/.rels",
                r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Montos" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Canal</t></is></c><c r="B1" t="inlineStr"><is><t>Tope</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Yape</t></is></c><c r="B2"><v>500</v></c></row></sheetData></worksheet>"#,
            ),
        ];
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            for (name, body) in files {
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn indexa_hojas_y_filas() {
        let text = spreadsheet_text(&xlsx(), 10_000).unwrap();
        assert_eq!(text, "Hoja: Montos\nCanal | Tope\nYape | 500\n");
    }

    #[test]
    fn guarda_fila_y_columna_de_cada_celda() {
        let e = spreadsheet_extract(&xlsx(), 10_000).unwrap();
        assert_eq!(e.layout, "\nr1 A,B\nr2 A,B\n");
        assert_eq!(e.text.lines().count(), e.layout.lines().count());
        assert!(!e.truncated);
        assert!(spreadsheet_extract(&xlsx(), 20).unwrap().truncated);
    }

    #[test]
    fn letras_de_columna() {
        assert_eq!(column_letters(0), "A");
        assert_eq!(column_letters(25), "Z");
        assert_eq!(column_letters(26), "AA");
        assert_eq!(column_letters(27), "AB");
        assert_eq!(column_letters(701), "ZZ");
        assert_eq!(column_letters(702), "AAA");
    }
}
