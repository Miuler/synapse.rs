//! Caracterización de las herramientas MCP: congela la respuesta de cada herramienta sobre una
//! bóveda de ejemplo para que el refactor a casos de uso no cambie ningún resultado ni mensaje.
//! Regenerar solo al crearla: `UPDATE_CHARACTERIZATION=1 cargo test --test mcp_characterization_tests -- --nocapture`.
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::fs;
use std::sync::Arc;

fn write(vault: &std::path::Path, rel: &str, content: &str) {
    let p = vault.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

fn build_vault(vault: &std::path::Path) {
    write(
        vault,
        "arquitectura.md",
        "---\ntitle: Arquitectura Synapse\ntags: [rust, tauri, onion]\naliases: [arq]\ndescription: Capas del backend\n---\n# Arquitectura\nDetalles del backend. Ver [[agentes]] y [[docs/fsd]].\n\n## Capas\nDominio, aplicación e infraestructura.\n\n### Dominio\nEntidades puras.\n\n## Diagramas\n```mermaid\ngraph TD\nA-->B\n```\n```d2\nx -> y\n```\nFin.\n",
    );
    write(
        vault,
        "agentes.md",
        "---\ntags: [rust, mcp]\n---\n# Agentes\nProtocolo MCP implementado. #ia/agentes\nEnlace a [[arquitectura]].\n",
    );
    write(
        vault,
        "docs/fsd.md",
        "---\ntags: [front, onion, rust]\n---\n# Feature Sliced Design\nCapas del front.\n",
    );
    write(
        vault,
        "docs/notas/idea.md",
        "# Idea\nUna idea suelta con #rust\n",
    );
    write(vault, "docs/diagrama.mmd", "graph LR\nX-->Y\n");
    write(vault, "recetas/pan.md", "# Pan\nHarina, agua y sal.\n");
    fs::create_dir_all(vault.join("vacia")).unwrap();
}

fn handler(
    vault: &std::path::Path,
) -> McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl> {
    McpToolHandler::new(
        vault.to_path_buf(),
        NoteRepositoryImpl::new(),
        None::<Arc<dyn FullTextIndex>>,
        None::<Arc<VersionControlImpl>>,
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    )
}

fn call(
    h: &McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl>,
    name: &str,
    args: Value,
    out: &mut String,
) {
    let res = h.call_tool(name, Some(args.clone()));
    out.push_str(&format!(
        "### {name} {args}\n{}\n",
        serde_json::to_string_pretty(&res).unwrap()
    ));
}

#[test]
fn mcp_tools_output_is_preserved() {
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path();
    build_vault(vault);
    let h = handler(vault);
    let mut out = String::new();

    // Lecturas
    call(&h, "search_notes", json!({"query": "arq"}), &mut out);
    call(
        &h,
        "search_notes",
        json!({"query": "", "tags": ["rust"]}),
        &mut out,
    );
    call(&h, "search_folders", json!({"query": "docs"}), &mut out);
    call(
        &h,
        "search_folders",
        json!({"limit": 2, "offset": 1}),
        &mut out,
    );
    call(&h, "full_text_search", json!({"query": "capas"}), &mut out);
    call(
        &h,
        "search_in_note",
        json!({"relative_path": "arquitectura.md", "query": "capas", "context_lines": 1}),
        &mut out,
    );
    call(
        &h,
        "list_folder_contents",
        json!({"folder_path": "docs"}),
        &mut out,
    );
    call(
        &h,
        "list_folder_contents",
        json!({"folder_path": "", "recursive": true, "include_folders": false}),
        &mut out,
    );
    call(
        &h,
        "list_folder_contents",
        json!({"folder_path": "no-existe"}),
        &mut out,
    );
    call(
        &h,
        "list_vault_tree",
        json!({"parent_path": "docs"}),
        &mut out,
    );
    call(
        &h,
        "list_sibling_notes",
        json!({"relative_path": "docs/fsd.md"}),
        &mut out,
    );
    call(
        &h,
        "read_note",
        json!({"relative_path": "arquitectura.md"}),
        &mut out,
    );
    call(
        &h,
        "read_note",
        json!({"relative_path": "arquitectura.md", "mode": "metadata_only"}),
        &mut out,
    );
    call(
        &h,
        "read_note",
        json!({"relative_path": "arquitectura.md", "section": "capas", "include_line_numbers": true}),
        &mut out,
    );
    call(
        &h,
        "read_note",
        json!({"relative_path": "arquitectura.md", "section": "nada"}),
        &mut out,
    );
    call(
        &h,
        "read_note",
        json!({"relative_path": "arquitectura.md", "mode": "chunk", "start_line": 3, "line_count": 4}),
        &mut out,
    );
    call(
        &h,
        "read_note",
        json!({"relative_path": "no-existe.md"}),
        &mut out,
    );
    call(
        &h,
        "get_backlinks_and_relations",
        json!({"relative_path": "arquitectura.md"}),
        &mut out,
    );
    call(&h, "get_vault_git_status", json!({}), &mut out);
    call(
        &h,
        "read_pdf_document",
        json!({"relative_path": "no.pdf"}),
        &mut out,
    );
    call(
        &h,
        "extract_note_diagrams",
        json!({"relative_path": "arquitectura.md"}),
        &mut out,
    );
    call(
        &h,
        "extract_note_diagrams",
        json!({"relative_path": "arquitectura.md", "diagram_type": "d2"}),
        &mut out,
    );
    call(
        &h,
        "extract_note_diagrams",
        json!({"relative_path": "docs/diagrama.mmd"}),
        &mut out,
    );
    call(&h, "herramienta_inexistente", json!({}), &mut out);

    // Escrituras (primero en seco)
    call(
        &h,
        "write_note",
        json!({"relative_path": "nueva.md", "content": "Hola", "title": "Nueva", "tags": ["x"], "dry_run": true}),
        &mut out,
    );
    call(
        &h,
        "write_note",
        json!({"relative_path": "nueva.md", "content": "Hola", "title": "Nueva", "tags": ["x"]}),
        &mut out,
    );
    call(
        &h,
        "write_note",
        json!({"relative_path": "nueva.md", "content": "Otra", "expected_hash": "sha256:malo"}),
        &mut out,
    );
    call(
        &h,
        "write_note",
        json!({"relative_path": "carpeta-nueva/nota.md", "content": "Hola", "title": "Nota"}),
        &mut out,
    );
    call(
        &h,
        "write_note",
        json!({"relative_path": "carpeta-nueva/nota.md", "content": "Otra"}),
        &mut out,
    );
    call(
        &h,
        "write_note",
        json!({"relative_path": "carpeta-nueva/nota.md", "content": "Otra", "overwrite": true}),
        &mut out,
    );
    call(
        &h,
        "write_note",
        json!({"relative_path": "carpeta-nueva/nota.md", "content": "Con hash", "expected_hash": "sha256:df650caabe31705662a3d08d1c32f1e05dc57beb917d6861489e30b51e3d5e9d"}),
        &mut out,
    );
    call(
        &h,
        "write_note",
        json!({"relative_path": "no-existe.md", "content": "Otra", "expected_hash": "sha256:malo"}),
        &mut out,
    );
    call(
        &h,
        "patch_note",
        json!({"relative_path": "recetas/pan.md", "old_text": "agua", "new_text": "leche", "dry_run": true}),
        &mut out,
    );
    call(
        &h,
        "patch_note",
        json!({"relative_path": "recetas/pan.md", "old_text": "agua", "new_text": "leche"}),
        &mut out,
    );
    call(
        &h,
        "patch_note",
        json!({"relative_path": "recetas/pan.md", "old_text": "no-esta", "new_text": "x"}),
        &mut out,
    );
    call(
        &h,
        "append_to_note",
        json!({"relative_path": "arquitectura.md", "content": "Nueva línea", "section_heading": "Capas"}),
        &mut out,
    );
    call(
        &h,
        "append_to_note",
        json!({"relative_path": "arquitectura.md", "content": "Al inicio", "section_heading": "Capas", "position": "start_of_section"}),
        &mut out,
    );
    call(
        &h,
        "append_to_note",
        json!({"relative_path": "arquitectura.md", "content": "x", "section_heading": "Nada"}),
        &mut out,
    );
    call(
        &h,
        "update_note_metadata",
        json!({"relative_path": "agentes.md", "title": "Agentes IA", "add_tags": ["ia"], "remove_tags": ["mcp"]}),
        &mut out,
    );
    call(
        &h,
        "read_note",
        json!({"relative_path": "arquitectura.md"}),
        &mut out,
    );

    // Movimientos y borrado
    call(
        &h,
        "move_notes",
        json!({"notes": ["docs/notas/idea.md"], "target_folder": "ideas", "update_wikilinks": true}),
        &mut out,
    );
    call(
        &h,
        "move_notes",
        json!({"notes": ["no-existe.md"], "target_folder": "ideas"}),
        &mut out,
    );
    call(
        &h,
        "move_folder_contents",
        json!({"source_folder": "docs", "target_folder": "documentos", "dry_run": true}),
        &mut out,
    );
    call(
        &h,
        "move_folder_contents",
        json!({"source_folder": "docs", "target_folder": "documentos"}),
        &mut out,
    );
    call(
        &h,
        "move_folder_contents",
        json!({"source_folder": "a", "target_folder": "a/b"}),
        &mut out,
    );
    call(
        &h,
        "delete_note",
        json!({"relative_path": "nueva.md", "expected_hash": "sha256:malo"}),
        &mut out,
    );
    call(
        &h,
        "delete_note",
        json!({"relative_path": "nueva.md"}),
        &mut out,
    );
    call(&h, "delete_note", json!({"relative_path": "  "}), &mut out);
    call(
        &h,
        "list_folder_contents",
        json!({"folder_path": "", "recursive": true}),
        &mut out,
    );

    let out = out.replace(&vault.to_string_lossy().to_string(), "<VAULT>");
    if std::env::var("UPDATE_CHARACTERIZATION").is_ok() {
        fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/characterization/mcp_tools.txt"
            ),
            &out,
        )
        .unwrap();
        return;
    }
    let expected = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/characterization/mcp_tools.txt"
    ))
    .unwrap();
    assert_eq!(
        out, expected,
        "la respuesta de alguna herramienta MCP cambió"
    );
}
