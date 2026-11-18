//! Herramientas del MCP con diagramas de draw.io (también dentro de un PNG) y modelos ArchiMate:
//! read_note, extract_note_diagrams, render_diagram de una imagen, inspect_archimate_element, y los
//! filtros extensions / max_matches de las búsquedas.
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;

type Handler = McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl>;

const MXFILE: &str = r#"<mxfile><diagram name="Flujo"><mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="g" value="Receptor" vertex="1" parent="1"><mxGeometry x="0" y="0" width="400" height="300" as="geometry"/></mxCell><mxCell id="a" value="Controller QTI1" vertex="1" parent="1"><mxGeometry x="10" y="10" width="100" height="40" as="geometry"/></mxCell><mxCell id="b" value="Kafka" vertex="1" parent="1"><mxGeometry x="500" y="0" width="80" height="40" as="geometry"/></mxCell><mxCell id="e" value="publica" edge="1" parent="1" source="a" target="b"/></root></mxGraphModel></diagram></mxfile>"#;

const ARCHIMATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<archimate:model xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:archimate="http://www.archimatetool.com/archimate" name="Modelo" id="id-1">
  <folder name="Application" id="id-2" type="application">
    <element xsi:type="archimate:ApplicationComponent" name="ACH Operation Maintenance Outbound V1" id="id-3">
      <property key="UNIQUECODE" value="CUD-7"/>
    </element>
    <element xsi:type="archimate:ApplicationService" name="Servicio de pagos" id="id-4"/>
  </folder>
  <folder name="Relations" id="id-5" type="relations">
    <element xsi:type="archimate:ServingRelationship" id="id-6" source="id-3" target="id-4"/>
  </folder>
  <folder name="Views" id="id-7" type="diagrams">
    <element xsi:type="archimate:ArchimateDiagramModel" name="Vista general" id="id-8">
      <child xsi:type="archimate:DiagramObject" id="id-9" archimateElement="id-3"/>
    </element>
  </folder>
</archimate:model>
"#;

/// PNG de 1×1 con el diagrama en un bloque `tEXt` (como lo guarda draw.io).
fn drawio_png() -> Vec<u8> {
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut c = (data.len() as u32).to_be_bytes().to_vec();
        c.extend_from_slice(kind);
        c.extend_from_slice(data);
        c.extend_from_slice(&[0; 4]);
        c
    }
    let encoded: String = MXFILE
        .bytes()
        .map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect();
    let mut text = b"mxfile\0".to_vec();
    text.extend_from_slice(encoded.as_bytes());
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend(chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]));
    png.extend(chunk(b"tEXt", &text));
    png.extend(chunk(b"IDAT", &[0]));
    png.extend(chunk(b"IEND", &[]));
    png
}

fn vault() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::create_dir_all(d.join("src/soluciones")).unwrap();
    std::fs::write(d.join("componentes.drawio.png"), drawio_png()).unwrap();
    std::fs::write(d.join("foto.png"), [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();
    std::fs::write(d.join("src/soluciones/modelo.archimate"), ARCHIMATE).unwrap();
    std::fs::write(d.join("nota.md"), "# Nota\n\nuno\nuno\nuno\n").unwrap();
    dir
}

fn handler(vault: &Path) -> Handler {
    McpToolHandler::new(
        vault.to_path_buf(),
        NoteRepositoryImpl::new(),
        None::<Arc<dyn FullTextIndex>>,
        None,
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    )
}

fn call(h: &Handler, name: &str, args: Value) -> Value {
    serde_json::to_value(h.call_tool(name, Some(args))).unwrap()
}

fn text_json(r: &Value) -> Value {
    assert_eq!(r["isError"], false, "{r}");
    serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn drawio_dentro_de_un_png_se_lee_y_se_estructura() {
    let dir = vault();
    let h = handler(dir.path());
    let read = text_json(&call(&h, "read_note", json!({ "relative_path": "componentes.drawio.png" })));
    assert_eq!(read["diagram"]["type"], "drawio");
    assert!(read["diagram"]["text"].as_str().unwrap().contains("Controller QTI1"));

    let structure = text_json(&call(&h, "extract_note_diagrams", json!({ "relative_path": "componentes.drawio.png" })));
    let page = &structure["pages"][0];
    assert_eq!(page["name"], "Flujo");
    assert_eq!(page["containers"][0]["title"], "Receptor");
    assert_eq!(page["containers"][0]["labels"][0], "Controller QTI1");
    assert_eq!(page["edges"][0], json!({ "from": "Controller QTI1", "to": "Kafka", "label": "publica" }));

    // Una imagen sin diagrama: read_note indica render_diagram, que la devuelve como imagen
    let plain = call(&h, "read_note", json!({ "relative_path": "foto.png" }));
    assert_eq!(plain["isError"], true);
    assert!(plain["content"][0]["text"].as_str().unwrap().contains("render_diagram"));
    let image = call(&h, "render_diagram", json!({ "relative_path": "foto.png" }));
    assert_eq!(image["isError"], false, "{image}");
    assert_eq!(image["content"][1]["type"], "image");
}

#[test]
fn archimate_indice_e_inspeccion() {
    let dir = vault();
    let h = handler(dir.path());
    let rel = "src/soluciones/modelo.archimate";
    let outline = text_json(&call(&h, "read_note", json!({ "relative_path": rel, "mode": "outline_only" })));
    let toc: Vec<String> = outline["toc"].as_array().unwrap().iter().map(|e| e["text"].as_str().unwrap().to_string()).collect();
    assert_eq!(toc, vec!["Application", "Relations", "Views", "Vista general"]);

    let found = text_json(&call(&h, "inspect_archimate_element", json!({ "relative_path": rel, "query": "maintenance" })));
    let e = &found["elements"][0];
    assert_eq!(e["id"], "id-3");
    assert_eq!(e["properties"][0], json!(["UNIQUECODE", "CUD-7"]));
    assert_eq!(e["outgoing"][0]["name"], "Servicio de pagos");
    assert_eq!(e["views"][0], "Vista general");
}

#[test]
fn search_notes_con_extensiones_y_search_in_note_con_tope() {
    let dir = vault();
    let h = handler(dir.path());
    // Sin extensiones solo Markdown; con ["archimate"] encuentra el modelo por su nombre
    let md_only = text_json(&call(&h, "search_notes", json!({ "query": "modelo" })));
    assert_eq!(md_only.as_array().unwrap().len(), 0);
    let models = text_json(&call(&h, "search_notes", json!({ "query": "modelo", "extensions": [".ArchiMate"] })));
    assert_eq!(models[0]["relative_path"], "src/soluciones/modelo.archimate");

    let capped = text_json(&call(&h, "search_in_note", json!({ "relative_path": "nota.md", "query": "uno", "max_matches": 2 })));
    assert_eq!(capped["total_matches"], 3);
    assert_eq!(capped["returned_matches"], 2);
}
