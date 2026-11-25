//! Buscar archivos y líneas como `fd`/`rg` (`find_files`, `grep_vault`) y las pistas cuando una
//! ruta no existe en la bóveda activa o `search_notes` no encuentra Markdown.
use app_lib::application::service::vault_catalog::{VaultCatalog, VaultRef};
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct InMemoryVaultCatalogImpl(Vec<VaultRef>);
impl VaultCatalog for InMemoryVaultCatalogImpl {
    fn list(&self) -> Vec<VaultRef> {
        self.0.clone()
    }
}

type Handler = McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl>;

fn write(root: &Path, rel: &str, content: &[u8]) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

fn vault(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    fs::create_dir_all(&p).unwrap();
    p.canonicalize().unwrap()
}

fn handler(here: &Path, vaults: Vec<VaultRef>) -> Handler {
    McpToolHandler::new(
        here.to_path_buf(),
        NoteRepositoryImpl::new(),
        None::<Arc<dyn FullTextIndex>>,
        None::<Arc<VersionControlImpl>>,
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    )
    .with_vaults(Some(Arc::new(InMemoryVaultCatalogImpl(vaults))), None)
}

/// Texto de todos los bloques de la respuesta y si es error.
fn call(h: &Handler, name: &str, args: Value) -> (String, bool) {
    let r = h.call_tool(name, Some(args));
    let text = r
        .content
        .iter()
        .filter_map(|c| c.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    (text, r.is_error)
}

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn sample(root: &Path) {
    write(root, "README.md", b"# Repo\n");
    write(
        root,
        "migrations/020_schema_ach_operation_ddl.sql",
        b"CREATE TABLE ach_operating_session (\n  id uuid,\n  status text\n);\nCREATE TABLE other (x int);\n",
    );
    write(
        root,
        "src/main/java/infrastructure/entity/OperationMasterEntity.java",
        b"@Table(name = \"master_transaction\")\nclass OperationMasterEntity {}\n",
    );
    write(root, "src/sequences/QTI1.mmd", b"sequenceDiagram\n  MS->>TIEL: BCTITPE8\n");
    write(root, "src/soluciones/SBB-SD-000360.archimate", b"<element name=\"ACH Operation Maintenance Outbound V1\" id=\"id-ff76\"/>\n");
    write(root, "target/classes/Generated.java", b"entity TIEL\n");
    write(root, ".git/config", b"TIEL\n");
    write(root, "img/logo.png", b"\x89PNG\x00\x00TIEL");
}

#[test]
fn find_files_con_regex_glob_y_extensiones() {
    let tmp = tempfile::tempdir().unwrap();
    let v = vault(tmp.path(), "repo");
    sample(&v);
    let h = handler(&v, vec![]);

    let (text, err) = call(&h, "find_files", json!({"regex": r"entity|model|mapper|\.sql|repository"}));
    assert!(!err, "{text}");
    let r = json_of(&text);
    let paths: Vec<&str> = r["files"].as_array().unwrap().iter().map(|f| f["path"].as_str().unwrap()).collect();
    assert_eq!(
        paths,
        [
            "migrations/020_schema_ach_operation_ddl.sql",
            "src/main/java/infrastructure/entity/OperationMasterEntity.java"
        ],
        "omite target/ y ocultos"
    );

    let (text, _) = call(&h, "find_files", json!({"glob": "*ddl*", "extensions": ["sql"]}));
    assert_eq!(json_of(&text)["total_matches"], 1);

    let (text, _) = call(&h, "find_files", json!({"extensions": [".ArchiMate"]}));
    assert_eq!(json_of(&text)["files"][0]["path"], "src/soluciones/SBB-SD-000360.archimate");

    let (text, _) = call(&h, "find_files", json!({"folder_path": "src", "limit": 1}));
    let r = json_of(&text);
    assert_eq!(r["total_matches"], 3);
    assert_eq!(r["next_offset"], 1);

    let (text, err) = call(&h, "find_files", json!({"regex": "(sin cerrar"}));
    assert!(err && text.contains("no válida"), "{text}");
}

#[test]
fn grep_vault_como_rg() {
    let tmp = tempfile::tempdir().unwrap();
    let v = vault(tmp.path(), "repo");
    sample(&v);
    let h = handler(&v, vec![VaultRef { name: "repo".into(), path: v.clone(), open: true }]);

    let (text, err) = call(&h, "grep_vault", json!({"pattern": "tiel|BCTITPE"}));
    assert!(!err, "{text}");
    let r = json_of(&text);
    assert_eq!(r["total_matches"], 1, "ni target/, ni .git, ni el PNG binario: {text}");
    assert_eq!(r["files"][0]["path"], "src/sequences/QTI1.mmd");
    assert_eq!(r["files"][0]["matches"][0]["line"], 2);

    let (text, _) = call(
        &h,
        "grep_vault",
        json!({"pattern": "CREATE TABLE.*session", "context_after": 2, "glob": "*.sql"}),
    );
    let m = &json_of(&text)["files"][0]["matches"][0];
    assert_eq!(m["after"], json!(["  id uuid,", "  status text"]));

    let (text, _) = call(&h, "grep_vault", json!({"pattern": "id-ff76", "files_only": true, "vault": "repo"}));
    let r = json_of(&text);
    assert_eq!(r["files"][0]["path"], "src/soluciones/SBB-SD-000360.archimate");
    assert!(r["files"][0].get("matches").is_none());
}

#[test]
fn ruta_inexistente_dice_la_boveda_activa_y_donde_si_esta() {
    let tmp = tempfile::tempdir().unwrap();
    let notas = vault(tmp.path(), "MisNotas");
    let emisor = vault(tmp.path(), "ds-emisor");
    write(&emisor, "src/sequences/Retry.mmd", b"sequenceDiagram\n");
    write(&notas, "otros/Retry.mmd", b"sequenceDiagram\n");
    let vaults = vec![
        VaultRef { name: "MisNotas".into(), path: notas.clone(), open: true },
        VaultRef { name: "ds-emisor".into(), path: emisor.clone(), open: true },
    ];
    let h = handler(&notas, vaults);

    let (text, err) = call(&h, "read_note", json!({"relative_path": "src/sequences/Retry.mmd"}));
    assert!(err);
    assert!(text.contains("bóveda activa «MisNotas»"), "{text}");
    assert!(text.contains("Sí existe en: «ds-emisor»"), "{text}");
    assert!(text.contains("otros/Retry.mmd"), "mismo nombre en esta bóveda: {text}");

    let (text, err) = call(&h, "list_folder_contents", json!({"folder_path": "src/sequences"}));
    assert!(err && text.contains("vault: \"ds-emisor\""), "{text}");

    // Una ruta que sí existe no añade nada
    let (text, _) = call(&h, "read_note", json!({"relative_path": "otros/Retry.mmd"}));
    assert!(!text.contains("bóveda activa"));
}

#[test]
fn search_notes_vacio_avisa_de_otros_tipos() {
    let tmp = tempfile::tempdir().unwrap();
    let v = vault(tmp.path(), "repo");
    sample(&v);
    let h = handler(&v, vec![]);

    let (text, err) = call(&h, "search_notes", json!({"query": "archimate"}));
    assert!(!err);
    assert!(text.starts_with("[]"), "el resultado sigue siendo la lista: {text}");
    assert!(text.contains("SBB-SD-000360.archimate") && text.contains("find_files"), "{text}");

    // Con extensions explícitas no hay aviso
    let (text, _) = call(&h, "search_notes", json!({"query": "nada-de-nada", "extensions": ["md"]}));
    assert_eq!(text, "[]");
}
