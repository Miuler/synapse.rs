//! Diagramas por el MCP: `read_note` los marca y `render_diagram` los devuelve como imagen.
use app_lib::application::service::diagram_renderer::{DiagramRenderer, RenderedImage};
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::sync::Arc;

struct MockDiagramRendererImpl;

impl DiagramRenderer for MockDiagramRendererImpl {
    fn render(&self, relative_path: &str, content: Option<&str>) -> Result<RenderedImage, String> {
        Ok(RenderedImage {
            mime_type: "image/png".into(),
            data_base64: match content {
                Some(c) => format!("PNG:{relative_path}:{}", c.len()),
                None => format!("PNG:{relative_path}"),
            },
        })
    }
}

type Handler = McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl>;

fn handler(vault: &std::path::Path, renderer: bool) -> Handler {
    McpToolHandler::new(
        vault.to_path_buf(),
        NoteRepositoryImpl::new(),
        None::<Arc<dyn FullTextIndex>>,
        None::<Arc<VersionControlImpl>>,
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    )
    .with_diagram_renderer(
        renderer.then(|| Arc::new(MockDiagramRendererImpl) as Arc<dyn DiagramRenderer>),
    )
}

fn as_json(h: &Handler, name: &str, args: Value) -> Value {
    serde_json::to_value(h.call_tool(name, Some(args))).unwrap()
}

#[test]
fn read_note_marca_los_diagramas_y_render_diagram_devuelve_la_imagen() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("flujo.d2"), "a -> b\n").unwrap();
    std::fs::write(tmp.path().join("nota.md"), "# Nota\n").unwrap();
    let h = handler(tmp.path(), true);

    let read = as_json(&h, "read_note", json!({"relative_path": "flujo.d2"}));
    let payload: Value =
        serde_json::from_str(read["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["diagram"]["kind"], "d2");
    assert_eq!(payload["diagram"]["render_tool"], "render_diagram");
    assert_eq!(payload["diagram"]["renderable"], true);

    let nota = as_json(&h, "read_note", json!({"relative_path": "nota.md"}));
    let payload: Value =
        serde_json::from_str(nota["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(payload.get("diagram").is_none());

    let img = as_json(&h, "render_diagram", json!({"relative_path": "flujo.d2"}));
    assert_eq!(img["isError"], false);
    assert_eq!(img["content"][1]["type"], "image");
    assert_eq!(img["content"][1]["mimeType"], "image/png");
    assert_eq!(img["content"][1]["data"], "PNG:flujo.d2");
    assert!(img["content"][1].get("text").is_none());

    let no = as_json(&h, "render_diagram", json!({"relative_path": "nota.md"}));
    assert_eq!(no["isError"], true);
}

#[test]
fn sin_ventana_render_diagram_explica_que_falta() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("x.mmd"), "graph TD\nA-->B\n").unwrap();
    let h = handler(tmp.path(), false);
    let read = as_json(&h, "read_note", json!({"relative_path": "x.mmd"}));
    let payload: Value =
        serde_json::from_str(read["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["diagram"]["renderable"], false);
    let img = as_json(&h, "render_diagram", json!({"relative_path": "x.mmd"}));
    assert_eq!(img["isError"], true);
    assert!(
        img["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("window")
    );
}
