//! Texto de una planilla (Excel `.xlsx`/`.xls`/`.xlsm`, OpenDocument `.ods`) para el índice
//! full-text: el nombre de cada hoja y sus filas, con las celdas separadas por ` | `.

use calamine::{Data, Reader, open_workbook_auto_from_rs};
use std::io::Cursor;

pub fn spreadsheet_text(bytes: &[u8], max_chars: usize) -> Result<String, String> {
    let mut book = open_workbook_auto_from_rs(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut out = String::new();
    for name in book.sheet_names().to_vec() {
        let Ok(range) = book.worksheet_range(&name) else {
            continue;
        };
        out.push_str(&format!("Hoja: {name}\n"));
        for row in range.rows() {
            let cells: Vec<String> = row
                .iter()
                .filter(|c| !matches!(c, Data::Empty))
                .map(|c| c.to_string().trim().to_string())
                .filter(|c| !c.is_empty())
                .collect();
            if cells.is_empty() {
                continue;
            }
            out.push_str(&cells.join(" | "));
            out.push('\n');
            if out.len() >= max_chars {
                return Ok(out);
            }
        }
    }
    Ok(out)
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
}
