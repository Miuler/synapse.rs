//! Diff de documentos de Word en dos niveles: primero los bloques y, en los que cambiaron, las palabras
//! o las celdas.
use crate::domain::docx::{
    DocxBlock, DocxBlockChange, DocxCell, DocxCellChange, DocxChangeKind, DocxDiffEntry, DocxDiffer,
    DocxDocument, DocxRowChange, DocxSegment, DocxTable, DocxText, row_key,
};
use similar::{Algorithm, DiffOp, capture_diff_slices_deadline};
use std::collections::HashMap;
use std::ops::Range;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(4);
/// Parecido mínimo (0-1) para tratar dos bloques como el mismo, modificado.
const MIN_SIMILARITY: f32 = 0.4;
/// Parejas que se comparan como mucho en un tramo cambiado; por encima se emparejan por orden.
const MAX_PAIRS: usize = 40_000;

#[derive(Clone, Copy, Default)]
pub struct DocxDifferImpl;

struct Differ {
    deadline: Instant,
}

/// Tramos iguales y tramos cambiados, en orden.
enum Span {
    Equal(Range<usize>, Range<usize>),
    Changed(Range<usize>, Range<usize>),
}

fn spans<T: Eq + std::hash::Hash>(old: &[T], new: &[T], deadline: Instant) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for op in capture_diff_slices_deadline(Algorithm::Myers, old, new, Some(deadline)) {
        let (o, n) = (op.old_range(), op.new_range());
        match (&op, out.last_mut()) {
            (DiffOp::Equal { .. }, _) => out.push(Span::Equal(o, n)),
            (_, Some(Span::Changed(po, pn))) => {
                po.end = o.end.max(po.end);
                pn.end = n.end.max(pn.end);
            }
            _ => out.push(Span::Changed(o, n)),
        }
    }
    out
}

fn words(text: &str) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for w in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        *out.entry(w.to_lowercase()).or_insert(0) += 1;
    }
    out
}

/// Coeficiente de Dice sobre las palabras: 1 iguales, 0 nada en común.
fn similarity(a: &HashMap<String, usize>, b: &HashMap<String, usize>) -> f32 {
    let total: usize = a.values().sum::<usize>() + b.values().sum::<usize>();
    if total == 0 {
        return 0.0;
    }
    let common: usize = a
        .iter()
        .map(|(w, n)| (*n).min(b.get(w).copied().unwrap_or(0)))
        .sum();
    2.0 * common as f32 / total as f32
}

/// Empareja sin cruces, maximizando el parecido total. Devuelve, en orden, (viejo, nuevo).
fn align(
    old: usize,
    new: usize,
    score: impl Fn(usize, usize) -> f32,
) -> Vec<(Option<usize>, Option<usize>)> {
    if old * new > MAX_PAIRS {
        // Demasiado para buscar la mejor pareja: por posición
        let mut out = Vec::new();
        for i in 0..old.max(new) {
            let (o, n) = ((i < old).then_some(i), (i < new).then_some(i));
            match (o, n) {
                (Some(a), Some(b)) if score(a, b) < MIN_SIMILARITY => {
                    out.push((o, None));
                    out.push((None, n));
                }
                _ => out.push((o, n)),
            }
        }
        return out;
    }
    let w = new + 1;
    let mut best = vec![0f32; (old + 1) * w];
    let mut pair = vec![0f32; (old + 1) * w];
    for i in (0..old).rev() {
        for j in (0..new).rev() {
            let s = score(i, j);
            pair[i * w + j] = s;
            let mut b = best[(i + 1) * w + j].max(best[i * w + j + 1]);
            if s >= MIN_SIMILARITY {
                b = b.max(best[(i + 1) * w + j + 1] + s);
            }
            best[i * w + j] = b;
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < old && j < new {
        let s = pair[i * w + j];
        let here = best[i * w + j];
        if s >= MIN_SIMILARITY && here == best[(i + 1) * w + j + 1] + s {
            out.push((Some(i), Some(j)));
            i += 1;
            j += 1;
        } else if here == best[(i + 1) * w + j] {
            out.push((Some(i), None));
            i += 1;
        } else {
            out.push((None, Some(j)));
            j += 1;
        }
    }
    out.extend((i..old).map(|i| (Some(i), None)));
    out.extend((j..new).map(|j| (None, Some(j))));
    out
}

/// Palabras, espacios y signos por separado.
fn tokens(text: &str) -> Vec<&str> {
    let class = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            0
        } else if c.is_whitespace() {
            1
        } else {
            2
        }
    };
    let mut out = Vec::new();
    let mut start = 0;
    let mut prev = None;
    for (i, c) in text.char_indices() {
        let k = class(c);
        if i > start && (Some(k) != prev || k == 2) {
            out.push(&text[start..i]);
            start = i;
        }
        prev = Some(k);
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

impl Differ {
    fn blocks(&self, old: &[DocxBlock], new: &[DocxBlock]) -> Vec<DocxDiffEntry> {
        let ok: Vec<String> = old.iter().map(DocxBlock::key).collect();
        let nk: Vec<String> = new.iter().map(DocxBlock::key).collect();
        let mut out = Vec::new();
        for span in spans(&ok, &nk, self.deadline) {
            match span {
                Span::Equal(_, n) => out.extend(new[n].iter().cloned().map(|block| DocxDiffEntry::Equal { block })),
                Span::Changed(o, n) => self.changed(&old[o], &new[n], &mut out),
            }
        }
        out
    }

    fn changed(&self, old: &[DocxBlock], new: &[DocxBlock], out: &mut Vec<DocxDiffEntry>) {
        let ow: Vec<_> = old.iter().map(|b| words(&b.plain_text())).collect();
        let nw: Vec<_> = new.iter().map(|b| words(&b.plain_text())).collect();
        let pairs = align(old.len(), new.len(), |i, j| match (&old[i], &new[j]) {
            (DocxBlock::Text(a), DocxBlock::Text(b)) if a.text == b.text => 1.0,
            (DocxBlock::Text(_), DocxBlock::Text(_)) => similarity(&ow[i], &nw[j]),
            (DocxBlock::Table(_), DocxBlock::Table(_)) => similarity(&ow[i], &nw[j]),
            // Dos imágenes en el mismo tramo: se muestran como antes y después
            (DocxBlock::Image(_), DocxBlock::Image(_)) => 0.5,
            _ => 0.0,
        });
        for pair in pairs {
            out.push(match pair {
                (Some(i), Some(j)) => DocxDiffEntry::Modified {
                    change: self.change(&old[i], &new[j]),
                },
                (Some(i), None) => DocxDiffEntry::Removed { block: old[i].clone() },
                (None, Some(j)) => DocxDiffEntry::Added { block: new[j].clone() },
                (None, None) => continue,
            });
        }
    }

    fn change(&self, old: &DocxBlock, new: &DocxBlock) -> DocxBlockChange {
        match (old, new) {
            (DocxBlock::Table(a), DocxBlock::Table(b)) => DocxBlockChange::Table { rows: self.rows(a, b) },
            (DocxBlock::Image(a), DocxBlock::Image(b)) => DocxBlockChange::Image {
                old: a.clone(),
                new: b.clone(),
            },
            (DocxBlock::Text(a), DocxBlock::Text(b)) => self.text(a, b),
            // `changed` solo empareja bloques del mismo tipo
            (a, b) => self.text(
                &DocxText { text: a.plain_text(), ..DocxText::default() },
                &DocxText { text: b.plain_text(), ..DocxText::default() },
            ),
        }
    }

    fn text(&self, old: &DocxText, new: &DocxText) -> DocxBlockChange {
        DocxBlockChange::Text {
            old_heading: old.heading,
            new_heading: new.heading,
            list_level: new.list_level,
            segments: self.segments(&old.text, &new.text),
        }
    }

    fn segments(&self, old: &str, new: &str) -> Vec<DocxSegment> {
        let (ot, nt) = (tokens(old), tokens(new));
        let mut out: Vec<DocxSegment> = Vec::new();
        let mut pending = (String::new(), String::new());
        let flush = |pending: &mut (String, String), out: &mut Vec<DocxSegment>| {
            for (kind, text) in [
                (DocxChangeKind::Removed, std::mem::take(&mut pending.0)),
                (DocxChangeKind::Added, std::mem::take(&mut pending.1)),
            ] {
                if !text.is_empty() {
                    out.push(DocxSegment { kind, text });
                }
            }
        };
        let all = spans(&ot, &nt, self.deadline);
        let last = all.len().saturating_sub(1);
        for (i, span) in all.into_iter().enumerate() {
            match span {
                Span::Changed(o, n) => {
                    pending.0.push_str(&ot[o].concat());
                    pending.1.push_str(&nt[n].concat());
                }
                Span::Equal(_, n) => {
                    let text = nt[n].concat();
                    // Un espacio suelto entre dos cambios se funde con ellos: se lee mejor
                    let changing = !pending.0.is_empty() || !pending.1.is_empty();
                    if changing && i != last && text.chars().all(char::is_whitespace) {
                        pending.0.push_str(&text);
                        pending.1.push_str(&text);
                        continue;
                    }
                    flush(&mut pending, &mut out);
                    out.push(DocxSegment { kind: DocxChangeKind::Equal, text });
                }
            }
        }
        flush(&mut pending, &mut out);
        out
    }

    fn rows(&self, old: &DocxTable, new: &DocxTable) -> Vec<DocxRowChange> {
        let ok: Vec<String> = old.rows.iter().map(|r| row_key(r)).collect();
        let nk: Vec<String> = new.rows.iter().map(|r| row_key(r)).collect();
        let whole = |row: &[DocxCell], kind: DocxChangeKind| DocxRowChange {
            kind,
            cells: row.iter().map(|c| whole_cell(c, kind)).collect(),
        };
        let mut out = Vec::new();
        for span in spans(&ok, &nk, self.deadline) {
            match span {
                Span::Equal(_, n) => out.extend(new.rows[n].iter().map(|r| whole(r, DocxChangeKind::Equal))),
                Span::Changed(o, n) => {
                    let (o, n) = (&old.rows[o], &new.rows[n]);
                    let bag = |r: &Vec<DocxCell>| words(&r.iter().map(DocxCell::plain_text).collect::<Vec<_>>().join(" "));
                    let ow: Vec<_> = o.iter().map(bag).collect();
                    let nw: Vec<_> = n.iter().map(bag).collect();
                    for pair in align(o.len(), n.len(), |i, j| {
                        // La misma celda en la misma columna también cuenta como parecido
                        let same = o[i].iter().zip(&n[j]).filter(|(a, b)| a.key() == b.key()).count();
                        let cells = o[i].len().max(n[j].len()).max(1);
                        similarity(&ow[i], &nw[j]).max(same as f32 / cells as f32)
                    }) {
                        match pair {
                            (Some(i), Some(j)) => out.push(DocxRowChange {
                                kind: DocxChangeKind::Modified,
                                cells: self.cells(&o[i], &n[j]),
                            }),
                            (Some(i), None) => out.push(whole(&o[i], DocxChangeKind::Removed)),
                            (None, Some(j)) => out.push(whole(&n[j], DocxChangeKind::Added)),
                            (None, None) => {}
                        }
                    }
                }
            }
        }
        out
    }

    fn cells(&self, old: &[DocxCell], new: &[DocxCell]) -> Vec<DocxCellChange> {
        let pairs: Vec<_> = if old.len() == new.len() {
            (0..old.len()).map(|i| (Some(i), Some(i))).collect()
        } else {
            // Se añadió o quitó una columna: se alinean las celdas iguales
            let ok: Vec<String> = old.iter().map(DocxCell::key).collect();
            let nk: Vec<String> = new.iter().map(DocxCell::key).collect();
            let mut out = Vec::new();
            for span in spans(&ok, &nk, self.deadline) {
                match span {
                    Span::Equal(o, n) => out.extend(o.map(Some).zip(n.map(Some))),
                    Span::Changed(o, n) => {
                        for k in 0..o.len().max(n.len()) {
                            let at = |r: &Range<usize>| (k < r.len()).then(|| r.start + k);
                            out.push((at(&o), at(&n)));
                        }
                    }
                }
            }
            out
        };
        pairs
            .into_iter()
            .filter_map(|pair| match pair {
                (Some(i), Some(j)) if old[i].key() == new[j].key() => Some(whole_cell(&new[j], DocxChangeKind::Equal)),
                (Some(i), Some(j)) => Some(DocxCellChange {
                    kind: DocxChangeKind::Modified,
                    col_span: new[j].col_span,
                    merged: new[j].merged,
                    entries: self.blocks(&old[i].blocks, &new[j].blocks),
                }),
                (Some(i), None) => Some(whole_cell(&old[i], DocxChangeKind::Removed)),
                (None, Some(j)) => Some(whole_cell(&new[j], DocxChangeKind::Added)),
                (None, None) => None,
            })
            .collect()
    }
}

fn whole_cell(cell: &DocxCell, kind: DocxChangeKind) -> DocxCellChange {
    let entry = |block: DocxBlock| match kind {
        DocxChangeKind::Added => DocxDiffEntry::Added { block },
        DocxChangeKind::Removed => DocxDiffEntry::Removed { block },
        _ => DocxDiffEntry::Equal { block },
    };
    DocxCellChange {
        kind,
        col_span: cell.col_span,
        merged: cell.merged,
        entries: cell.blocks.iter().cloned().map(entry).collect(),
    }
}

impl DocxDiffer for DocxDifferImpl {
    fn diff(&self, old: &DocxDocument, new: &DocxDocument) -> (Vec<DocxDiffEntry>, bool) {
        let differ = Differ { deadline: Instant::now() + TIMEOUT };
        let entries = differ.blocks(&old.blocks, &new.blocks);
        (entries, Instant::now() > differ.deadline)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ooxml_parser::{OoxmlParser, tests::{blocks, docx, with_image}};
    use super::*;
    use crate::domain::docx::DocxParser;

    fn p(t: &str) -> String {
        format!("<w:p><w:r><w:t>{t}</w:t></w:r></w:p>")
    }

    fn diff(old: &str, new: &str) -> Vec<DocxDiffEntry> {
        DocxDifferImpl.diff(&DocxDocument { blocks: blocks(old) }, &DocxDocument { blocks: blocks(new) }).0
    }

    /// Resumen legible: `=` igual, `+`/`-` agregado o eliminado, `~[-x-]{+y+}` modificado.
    fn summary(entries: &[DocxDiffEntry]) -> Vec<String> {
        entries.iter().map(|e| match e {
            DocxDiffEntry::Equal { block } => format!("={}", block.plain_text()),
            DocxDiffEntry::Added { block } => format!("+{}", block.plain_text()),
            DocxDiffEntry::Removed { block } => format!("-{}", block.plain_text()),
            DocxDiffEntry::Modified { change } => match change {
                DocxBlockChange::Text { segments, .. } => format!("~{}", segments.iter().map(|s| match s.kind {
                    DocxChangeKind::Added => format!("{{+{}+}}", s.text),
                    DocxChangeKind::Removed => format!("[-{}-]", s.text),
                    _ => s.text.clone(),
                }).collect::<String>()),
                DocxBlockChange::Image { .. } => "~imagen".into(),
                DocxBlockChange::Table { rows } => format!("~{}", rows.iter().map(|r| {
                    let mark = match r.kind { DocxChangeKind::Equal => '=', DocxChangeKind::Added => '+', DocxChangeKind::Removed => '-', DocxChangeKind::Modified => '~' };
                    format!("{mark}({})", r.cells.iter().map(|c| summary(&c.entries).join(",")).collect::<Vec<_>>().join("|"))
                }).collect::<String>()),
            },
        }).collect()
    }

    #[test]
    fn iguales_salvo_los_rsid() {
        let d = diff(r#"<w:p w:rsidR="1"><w:r><w:t>Hola mundo</w:t></w:r></w:p>"#, r#"<w:p w:rsidR="2"><w:r><w:t>Hola </w:t></w:r><w:r><w:t>mundo</w:t></w:r></w:p>"#);
        assert_eq!(summary(&d), ["=Hola mundo"]);
    }

    #[test]
    fn palabras_cambiadas_en_un_parrafo() {
        let d = diff(&p("El pago vence el lunes próximo."), &p("El pago vence el martes próximo."));
        assert_eq!(summary(&d), ["~El pago vence el [-lunes-]{+martes+} próximo."]);
        let d = diff(&p("uno dos tres cuatro cinco seis"), &p("uno DOS TRES cuatro cinco seis"));
        assert_eq!(summary(&d), ["~uno [-dos tres-]{+DOS TRES+} cuatro cinco seis"]);
    }

    #[test]
    fn empareja_por_parecido_y_no_por_posicion() {
        let old = [p("El contrato dura dos años completos"), p("Firma de las partes interesadas")].concat();
        let new = [p("Texto totalmente distinto aquí"), p("El contrato dura tres años completos"), p("Firma de las partes")].concat();
        assert_eq!(summary(&diff(&old, &new)), [
            "+Texto totalmente distinto aquí",
            "~El contrato dura [-dos-]{+tres+} años completos",
            "~Firma de las partes[- interesadas-]",
        ]);
        assert_eq!(summary(&diff(&p("nada que ver"), &p("otra cosa distinta"))), ["-nada que ver", "+otra cosa distinta"]);
    }

    #[test]
    fn un_titulo_que_cambia_de_nivel() {
        let t = |n: u8| format!(r#"<w:p><w:pPr><w:outlineLvl w:val="{n}"/></w:pPr><w:r><w:t>Alcance</w:t></w:r></w:p>"#);
        let d = diff(&t(0), &t(1));
        assert!(matches!(&d[0], DocxDiffEntry::Modified { change: DocxBlockChange::Text { old_heading: Some(1), new_heading: Some(2), .. } }));
    }

    #[test]
    fn celdas_filas_y_columnas() {
        let table = |rows: &[&[&str]]| format!("<w:tbl>{}</w:tbl>", rows.iter().map(|r| format!("<w:tr>{}</w:tr>", r.iter().map(|c| format!("<w:tc>{}</w:tc>", p(c))).collect::<String>())).collect::<String>());
        let d = diff(&table(&[&["Campo", "Valor"], &["monto", "10"], &["moneda", "PEN"]]), &table(&[&["Campo", "Valor"], &["monto", "20"], &["plazo", "3 días"], &["moneda", "PEN"]]));
        assert_eq!(summary(&d), ["~=(=Campo|=Valor)~(=monto|-10,+20)+(+plazo|+3 días)=(=moneda|=PEN)"]);
        let d = diff(&table(&[&["a", "b"], &["c", "d"]]), &table(&[&["a", "nueva", "b"], &["c", "x", "d"]]));
        assert_eq!(summary(&d), ["~~(=a|+nueva|=b)~(=c|+x|=d)"]);
    }

    #[test]
    fn la_imagen_cambia_por_su_contenido() {
        let doc = |id: &str, name: &str, data: &[u8]| OoxmlParser.parse(&with_image(id, name, data)).unwrap();
        let same = DocxDifferImpl.diff(&doc("rId1", "a.png", b"gato"), &doc("rId8", "b.png", b"gato")).0;
        assert!(same.iter().all(|e| matches!(e, DocxDiffEntry::Equal { .. })));
        let changed = DocxDifferImpl.diff(&doc("rId1", "a.png", b"gato"), &doc("rId1", "a.png", b"perro")).0;
        assert_eq!(summary(&changed), ["=Antes", "~imagen"]);
    }

    #[test]
    fn documento_agregado_o_eliminado() {
        let empty = DocxDocument::default();
        let doc = OoxmlParser.parse(&docx(&p("hola"))).unwrap();
        assert_eq!(summary(&DocxDifferImpl.diff(&empty, &doc).0), ["+hola"]);
        assert_eq!(summary(&DocxDifferImpl.diff(&doc, &empty).0), ["-hola"]);
    }

    #[test]
    fn uno_grande_y_distinto_acaba_a_tiempo() {
        let many = |s: &str| DocxDocument { blocks: (0..4_000).map(|i| DocxBlock::Text(DocxText { text: format!("{s} {i} palabra {}", i * 7), ..DocxText::default() })).collect() };
        let t = Instant::now();
        let (d, _) = DocxDifferImpl.diff(&many("uno"), &many("dos"));
        assert!(!d.is_empty() && t.elapsed() < Duration::from_secs(15));
    }
}
