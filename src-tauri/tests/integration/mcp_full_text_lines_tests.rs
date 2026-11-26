//! `full_text_search` con alternativas (`a|b`), líneas que coinciden (`lines`) y modo exacto.
use app_lib::domain::vault::file_types::SupportedFileTypes;
use app_lib::domain::search::full_text::FullTextDocument;
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use app_lib::infrastructure::repository::tantivy::full_text_index_impl::FullTextIndexImpl;
use app_lib::infrastructure::search::content_extractor::extract;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::sync::Arc;

type Handler = McpToolHandler<NoteRepositoryImpl, FullTextIndexImpl, VersionControlImpl>;

/// Escribe los archivos en la bóveda y los indexa con los extractores reales.
fn vault(files: &[(&str, &str)]) -> (tempfile::TempDir, Handler) {
    let dir = tempfile::tempdir().unwrap();
    let index = FullTextIndexImpl::in_memory().unwrap();
    let types = SupportedFileTypes::default();
    let mut docs = Vec::new();
    for (rel, content) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        let (kind, title, body, headings) = extract(Path::new(rel), content.as_bytes(), &types).unwrap();
        docs.push(FullTextDocument { path: rel.to_string(), title, body, headings, kind, mtime_nanos: 1 });
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
    let mut p: Vec<String> = r["hits"].as_array().unwrap().iter().map(|h| h["path"].as_str().unwrap().to_string()).collect();
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
        assert_eq!(paths(&search(&h, json!({ "query": q }))), ["a.md", "b.md", "c.md"], "{q}");
    }
}

#[test]
fn lineas_con_el_numero_real_del_archivo_markdown() {
    let (_d, h) = vault(&[("docs/flujo.md", FLUJO)]);
    let r = search(&h, json!({ "query": "TIEL|QTI1", "lines": true, "context_lines": 1 }));
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
        ("src/sequences/outbound.mmd", "sequenceDiagram\n  APIM->>MS: POST /ach-operations/{id}/outbounds-ach/initiate\n"),
        ("src/sequences/otro.mmd", "sequenceDiagram\n  note over MS: outbounds ach sin guion\n"),
    ]);
    let normal = search(&h, json!({ "query": "outbounds-ach" }));
    assert_eq!(normal["total_matches"], 2, "sin exact, «outbounds ach» (sin guion) también coincide");
    let r = search(&h, json!({ "query": "outbounds-ach|/ach-operations/", "exact": true, "lines": true }));
    assert_eq!(paths(&r), ["src/sequences/outbound.mmd"]);
    assert_eq!(r["hits"][0]["lines"][0]["location"], json!({ "line": 2 }));
    let err = h.call_tool("full_text_search", Some(json!({ "query": ":", "exact": true })));
    assert!(err.is_error);
}
