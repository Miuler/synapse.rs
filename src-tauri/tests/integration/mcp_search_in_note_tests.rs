//! `search_in_note` con alternativas (`a|b`), recuentos y varias notas a la vez.
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::sync::Arc;

type Handler = McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl>;

fn handler(vault: &Path) -> Handler {
    McpToolHandler::new(
        vault.to_path_buf(),
        NoteRepositoryImpl::new(),
        None::<Arc<dyn FullTextIndex>>,
        None::<Arc<VersionControlImpl>>,
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    )
}

/// JSON de la respuesta (primer bloque de texto) y si es error.
fn call(h: &Handler, args: Value) -> (Value, String, bool) {
    let r = h.call_tool("search_in_note", Some(args));
    let all = r.content.iter().filter_map(|c| c.text.clone()).collect::<Vec<_>>().join("\n");
    let first = r.content.first().and_then(|c| c.text.clone()).unwrap_or_default();
    (serde_json::from_str(&first).unwrap_or(Value::Null), all, r.is_error)
}

fn vault() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src/sequences")).unwrap();
    fs::write(
        dir.path().join("mapping.md"),
        "# Mapping\n| campo | tipo |\n| id_ach_operation | uuid |\nLímite BD: 20\nOtro límite bd y LÍMITE BD\n",
    )
    .unwrap();
    fs::write(dir.path().join("src/sequences/QTI1.mmd"), "sequenceDiagram\n  MS->>TIEL: BCTITPE8\n").unwrap();
    fs::write(dir.path().join("src/sequences/QTI3.mmd"), "sequenceDiagram\n  MS->>tiel: BCTITPE4\n  MS->>DB: insert\n").unwrap();
    dir
}

#[test]
fn alternativas_con_matched_y_recuentos() {
    let dir = vault();
    let h = handler(dir.path());
    let (r, text, err) = call(
        &h,
        json!({"relative_path": "mapping.md", "query": "id_ach_operation|Límite BD|no almacena", "context_lines": 0}),
    );
    assert!(!err, "{text}");
    assert_eq!(r["alternatives"], json!(["id_ach_operation", "Límite BD", "no almacena"]));
    assert_eq!(r["total_matches"], 3);
    assert_eq!(r["matches"][1]["matched"], json!(["Límite BD"]));
    assert_eq!(r["counts"]["Límite BD"], json!({"lines": 2, "occurrences": 3}));
    assert_eq!(r["counts"]["no almacena"], json!({"lines": 0, "occurrences": 0}));
}

#[test]
fn sin_barras_la_respuesta_no_cambia_salvo_counts() {
    let dir = vault();
    let h = handler(dir.path());
    let (r, _, _) = call(&h, json!({"relative_path": "mapping.md", "query": "| campo |"}));
    assert!(r.get("alternatives").is_none());
    assert!(r["matches"][0].get("matched").is_none());
    assert_eq!(r["matches"][0]["line_number"], 2, "la fila de tabla se busca literal");
    assert_eq!(r["counts"]["| campo |"]["lines"], 1);
}

#[test]
fn counts_only_como_grep_c() {
    let dir = vault();
    let h = handler(dir.path());
    let (r, _, _) = call(&h, json!({"relative_path": "mapping.md", "query": "límite bd", "counts_only": true}));
    assert_eq!(r["total_matches"], 2);
    assert!(r.get("matches").is_none());
    assert_eq!(r["counts"]["límite bd"]["occurrences"], 3);
}

#[test]
fn varias_notas_y_una_que_no_existe() {
    let dir = vault();
    let h = handler(dir.path());
    let (r, text, err) = call(
        &h,
        json!({
            "relative_paths": ["src/sequences/QTI1.mmd", "QTI2.mmd", "src/sequences/QTI3.mmd"],
            "query": "TIEL|insert",
            "context_lines": 0
        }),
    );
    assert!(!err, "{text}");
    assert_eq!(r["total_matches"], 3);
    let files = r["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);
    assert_eq!(files[0]["total_matches"], 1);
    assert!(files[1]["error"].as_str().unwrap().contains("no existe en la bóveda activa"), "{text}");
    assert_eq!(files[2]["counts"]["insert"]["lines"], 1);
    assert_eq!(files[2]["matches"][0]["matched"], json!(["TIEL"]));
}

#[test]
fn errores_de_varias_notas() {
    let dir = vault();
    let h = handler(dir.path());
    let (_, _, err) = call(&h, json!({"relative_paths": ["a.md", "b.md"], "query": "x"}));
    assert!(err, "si ninguna nota existe, es error");
    let many: Vec<String> = (0..21).map(|i| format!("n{i}.md")).collect();
    let (_, text, err) = call(&h, json!({"relative_paths": many, "query": "x"}));
    assert!(err && text.contains("Como máximo 20"), "{text}");
    let (_, text, err) = call(&h, json!({"query": "x"}));
    assert!(err && text.contains("obligatorio"), "{text}");
}
