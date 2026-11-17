//! Herramientas de Git del MCP: historial, diff entre commits, lectura de una revisión e imagen de
//! un commit, con un archivo que se mueve de carpeta entre medio.
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

type Handler = McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl>;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Un PNG mínimo válido de 1×1.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn repo() -> (tempfile::TempDir, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    git(d, &["init", "-q"]);
    std::fs::create_dir_all(d.join("components")).unwrap();
    std::fs::write(d.join("components/flujo.md"), "# Flujo\n\nuno\ndos\n").unwrap();
    std::fs::write(d.join("components/diagrama.drawio.png"), PNG).unwrap();
    git(d, &["add", "."]);
    git(d, &["commit", "-qm", "primera versión"]);
    let first = git(d, &["rev-parse", "HEAD"]);
    std::fs::create_dir_all(d.join("src")).unwrap();
    git(d, &["mv", "components", "src/components"]);
    git(d, &["commit", "-qm", "mueve a src"]);
    std::fs::write(d.join("src/components/flujo.md"), "# Flujo\n\nuno\ntres\n").unwrap();
    git(d, &["commit", "-qam", "cambia dos por tres"]);
    let last = git(d, &["rev-parse", "HEAD"]);
    std::fs::write(
        d.join("src/components/flujo.md"),
        "# Flujo\n\nuno\ntres\ncuatro\n",
    )
    .unwrap();
    (dir, first, last)
}

fn handler(vault: &Path) -> Handler {
    McpToolHandler::new(
        vault.to_path_buf(),
        NoteRepositoryImpl::new(),
        None::<Arc<dyn FullTextIndex>>,
        Some(Arc::new(VersionControlImpl)),
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    )
}

fn call(h: &Handler, name: &str, args: Value) -> Value {
    let r = serde_json::to_value(h.call_tool(name, Some(args))).unwrap();
    assert_eq!(r["isError"], false, "{name}: {r}");
    r
}

fn text_json(r: &Value) -> Value {
    serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn historial_diff_y_revision_de_un_archivo_movido() {
    let (dir, first, last) = repo();
    let h = handler(dir.path());
    let rel = "src/components/flujo.md";

    let history = text_json(&call(&h, "git_file_history", json!({"relative_path": rel})));
    let commits = history["commits"].as_array().unwrap();
    assert_eq!(commits.len(), 3, "sigue el renombre: {history}");
    assert_eq!(commits[0]["message"], "cambia dos por tres");

    // Entre el primer commit (en components/) y el último
    let diff = text_json(&call(
        &h,
        "git_diff_revisions",
        json!({"relative_path": rel, "from": first, "to": last}),
    ));
    assert_eq!(diff["lines_added"], 1);
    assert_eq!(diff["lines_removed"], 1);
    assert!(diff["diff"].as_str().unwrap().contains("-dos"));
    assert!(diff["diff"].as_str().unwrap().contains("+tres"));

    // Sin `to`: contra la versión local del disco
    let local = text_json(&call(
        &h,
        "git_diff_revisions",
        json!({"relative_path": rel, "from": last}),
    ));
    assert_eq!(local["to"], "local");
    assert!(local["diff"].as_str().unwrap().contains("+cuatro"));

    // El archivo como era en el primer commit (con su nombre de entonces)
    let old = text_json(&call(
        &h,
        "git_read_revision",
        json!({"relative_path": rel, "revision": first}),
    ));
    assert_eq!(old["path_at_revision"], "components/flujo.md");
    assert!(old["content"].as_str().unwrap().contains("dos"));
}

#[test]
fn render_diagram_devuelve_la_imagen_de_un_commit() {
    let (dir, first, _) = repo();
    let h = handler(dir.path());
    let r = call(
        &h,
        "render_diagram",
        json!({"relative_path": "src/components/diagrama.drawio.png", "revision": first}),
    );
    assert_eq!(r["content"][1]["type"], "image");
    assert_eq!(r["content"][1]["mimeType"], "image/png");
}
