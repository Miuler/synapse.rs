//! `full_text_search` con alternativas (`a|b`), líneas que coinciden (`lines`) y modo exacto.
use app_lib::domain::config::config::IndexingConfig;
use app_lib::domain::search::full_text::FullTextDocument;
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::domain::vault::file_types::SupportedFileTypes;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use app_lib::infrastructure::repository::tantivy::full_text_index_impl::FullTextIndexImpl;
use app_lib::infrastructure::search::content_extractor::extract_document;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::sync::Arc;

type Handler = McpToolHandler<NoteRepositoryImpl, FullTextIndexImpl, VersionControlImpl>;

/// Escribe los archivos en la bóveda y los indexa con los extractores reales.
fn vault(files: &[(&str, &str)]) -> (tempfile::TempDir, Handler) {
    let bytes: Vec<(&str, &[u8])> = files.iter().map(|(p, c)| (*p, c.as_bytes())).collect();
    vault_bytes(&bytes)
}

/// Como `vault`, con archivos binarios (Word, planillas, PDF).
fn vault_bytes(files: &[(&str, &[u8])]) -> (tempfile::TempDir, Handler) {
    let dir = tempfile::tempdir().unwrap();
    let index = FullTextIndexImpl::in_memory().unwrap();
    let types = SupportedFileTypes::default();
    let mut docs = Vec::new();
    for (rel, content) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        let e =
            extract_document(Path::new(rel), content, &types, &IndexingConfig::default()).unwrap();
        docs.push(FullTextDocument {
            path: rel.to_string(),
            title: e.title,
            body: e.body,
            headings: e.headings,
            kind: e.kind,
            layout: e.layout,
            truncated: e.truncated,
            mtime_nanos: 1,
        });
    }
    index.upsert(docs).unwrap();
    index.commit().unwrap();
    let h = McpToolHandler::new(
        dir.path().to_path_buf(),
        NoteRepositoryImpl::new(),
        Some(Arc::new(index)),
        None::<Arc<VersionControlImpl>>,
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    );
    (dir, h)
}

fn search(h: &Handler, args: Value) -> Value {
    let r = h.call_tool("full_text_search", Some(args));
    let text = r.content[0].text.clone().unwrap();
    assert!(!r.is_error, "{text}");
    serde_json::from_str(&text).unwrap()
}

fn paths(r: &Value) -> Vec<String> {
    let mut p: Vec<String> = r["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["path"].as_str().unwrap().to_string())
        .collect();
    p.sort();
    p
}

const FLUJO: &str = "---\ntitle: Flujo emisor\ntags: [ach]\n---\n# Flujo emisor\n\nEl **MS** llama a TIEL con BCTITPE8.\n\n| campo | valor |\n|---|---|\n| interfaz | QTI1 |\n";

#[test]
fn alternativas_en_una_sola_consulta() {
    let (_d, h) = vault(&[
        ("a.md", "programa TIEL emisor"),
        ("b.md", "interfaz QTI1 receptor"),
        ("c.md", "otro QTI3"),
        ("d.md", "nada que ver"),
    ]);
    // Antes, por MCP, `TIEL OR QTI1 OR QTI3` no devolvía nada (el prefijo partía la consulta)
    for q in ["TIEL OR QTI1 OR QTI3", "TIEL|QTI1|QTI3"] {
        assert_eq!(
            paths(&search(&h, json!({ "query": q }))),
            ["a.md", "b.md", "c.md"],
            "{q}"
        );
    }
}

#[test]
fn lineas_con_el_numero_real_del_archivo_markdown() {
    let (_d, h) = vault(&[("docs/flujo.md", FLUJO)]);
    let r = search(
        &h,
        json!({ "query": "TIEL|QTI1", "lines": true, "context_lines": 1 }),
    );
    let hit = &r["hits"][0];
    assert_eq!(hit["total_line_matches"], 2);
    let lines = hit["lines"].as_array().unwrap();
    // Línea 7 del archivo (con frontmatter), no la del texto indexado
    assert_eq!(lines[0]["location"], json!({ "line": 7 }));
    assert_eq!(lines[0]["text"], "El **MS** llama a TIEL con BCTITPE8.");
    assert_eq!(lines[0]["before"], json!([""]));
    assert_eq!(lines[1]["location"], json!({ "line": 11 }));
    assert_eq!(r["lines_truncated"], false);
    // Sin `lines`, la respuesta no cambia
    let r = search(&h, json!({ "query": "TIEL" }));
    assert!(r["hits"][0].get("lines").is_none() && r.get("lines_truncated").is_none());
}

#[test]
fn lineas_con_la_ubicacion_de_cada_formato() {
    let archimate = r#"<?xml version="1.0" encoding="UTF-8"?>
<archimate:model xmlns:archimate="http://www.archimatetool.com/archimate" name="ACH">
  <folder name="Application" type="application">
    <element xsi:type="archimate:ApplicationComponent" name="ACH Operation Maintenance Outbound V1" id="id-ff76">
      <property key="UNIQUECODE" value="CUD-024684"/>
    </element>
  </folder>
</archimate:model>"#;
    let (_d, h) = vault(&[("src/soluciones/modelo.archimate", archimate)]);
    let r = search(&h, json!({ "query": "024684", "lines": true }));
    assert_eq!(
        r["hits"][0]["lines"][0]["location"],
        json!({ "element": "ACH Operation Maintenance Outbound V1", "type": "ApplicationComponent" })
    );
}

#[test]
fn modo_exacto_con_puntuacion_y_lineas() {
    let (_d, h) = vault(&[
        (
            "src/sequences/outbound.mmd",
            "sequenceDiagram\n  APIM->>MS: POST /ach-operations/{id}/outbounds-ach/initiate\n",
        ),
        (
            "src/sequences/otro.mmd",
            "sequenceDiagram\n  note over MS: outbounds ach sin guion\n",
        ),
    ]);
    let normal = search(&h, json!({ "query": "outbounds-ach" }));
    assert_eq!(
        normal["total_matches"], 2,
        "sin exact, «outbounds ach» (sin guion) también coincide"
    );
    let r = search(
        &h,
        json!({ "query": "outbounds-ach|/ach-operations/", "exact": true, "lines": true }),
    );
    assert_eq!(paths(&r), ["src/sequences/outbound.mmd"]);
    assert_eq!(r["hits"][0]["lines"][0]["location"], json!({ "line": 2 }));
    let err = h.call_tool(
        "full_text_search",
        Some(json!({ "query": ":", "exact": true })),
    );
    assert!(err.is_error);
}

/// Un ZIP con esos archivos (para `.docx` y `.xlsx`).
fn zip(files: &[(&str, &str)]) -> Vec<u8> {
    use std::io::Write;
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        for (name, body) in files {
            z.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(body.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }
    buf.into_inner()
}

fn docx() -> Vec<u8> {
    zip(&[
        (
            "word/styles.xml",
            r#"<w:styles xmlns:w="w"><w:style w:type="paragraph" w:styleId="Ttulo1"><w:name w:val="heading 1"/></w:style></w:styles>"#,
        ),
        (
            "word/document.xml",
            concat!(
                r#"<w:document xmlns:w="w"><w:body>"#,
                r#"<w:p><w:r><w:t>Especificación</w:t></w:r></w:p>"#,
                r#"<w:p><w:pPr><w:pStyle w:val="Ttulo1"/></w:pPr><w:r><w:t>3.2 Contrato de salida</w:t></w:r></w:p>"#,
                r#"<w:p><w:r><w:t>Se invoca TIEL con BCTITPE8</w:t></w:r></w:p>"#,
                r#"</w:body></w:document>"#
            ),
        ),
    ])
}

fn xlsx() -> Vec<u8> {
    zip(&[
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
            r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Mapping" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Campo</t></is></c></row><row r="37"><c r="B37" t="inlineStr"><is><t>achOperationId</t></is></c><c r="D37" t="inlineStr"><is><t>TIEL</t></is></c></row></sheetData></worksheet>"#,
        ),
    ])
}

fn pdf(pages: &[&str]) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" },
    );
    let resources_id = doc.add_object(dictionary! { "Font" => dictionary! { "F1" => font_id } });
    let mut kids = Vec::new();
    for text in pages {
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Tj", vec![Object::string_literal(*text)]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        kids.push(
            doc.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id, "Contents" => content_id, "Resources" => resources_id,
            })
            .into(),
        );
    }
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer).unwrap();
    buffer
}

const DRAWIO: &str = r#"<mxfile><diagram name="Vista general"><mxGraphModel><root><mxCell id="0"/><mxCell id="2" value="Pagos" vertex="1"/></root></mxGraphModel></diagram><diagram name="Datos"><mxGraphModel><root><mxCell id="0"/><mxCell id="2" value="DB ACH Outbound" vertex="1"/></root></mxGraphModel></diagram></mxfile>"#;

#[test]
fn ubicacion_en_word_planillas_pdf_y_draw_io() {
    let (docx, xlsx, pdf) = (
        docx(),
        xlsx(),
        pdf(&["Portada", "Indice", "Flujo con TIEL"]),
    );
    let (_d, h) = vault_bytes(&[
        ("spec.docx", &docx),
        ("map.xlsx", &xlsx),
        ("manual.pdf", &pdf),
        ("comp.drawio", DRAWIO.as_bytes()),
    ]);
    let r = search(&h, json!({ "query": "TIEL|outbound", "lines": true }));
    let location = |path: &str| {
        r["hits"]
            .as_array()
            .unwrap()
            .iter()
            .find(|hit| hit["path"] == path)
            .map(|hit| hit["lines"][0]["location"].clone())
            .unwrap_or_else(|| panic!("sin {path}: {r}"))
    };
    assert_eq!(
        location("spec.docx"),
        json!({ "section": "3.2 Contrato de salida", "paragraph": 3 })
    );
    assert_eq!(
        location("map.xlsx"),
        json!({ "sheet": "Mapping", "row": 37, "column": "D" })
    );
    assert_eq!(location("manual.pdf"), json!({ "page": 3 }));
    assert_eq!(location("comp.drawio"), json!({ "diagram": "Datos" }));
    // `hs:` ya filtra por los títulos de Word
    assert_eq!(
        paths(&search(&h, json!({ "query": "hs:contrato" }))),
        ["spec.docx"]
    );
}
