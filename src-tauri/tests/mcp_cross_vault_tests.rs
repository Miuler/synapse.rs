//! Consultar otra bóveda por el MCP: `list_vaults` y el parámetro `vault` (solo lectura, con permiso).
use app_lib::application::service::vault_catalog::{UserPrompter, VaultCatalog, VaultRef};
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use app_lib::infrastructure::repository::tantivy::full_text_index_impl::FullTextIndexImpl;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct InMemoryVaultCatalogImpl(Vec<VaultRef>);
impl VaultCatalog for InMemoryVaultCatalogImpl {
    fn list(&self) -> Vec<VaultRef> {
        self.0.clone()
    }
}

struct MockUserPrompterImpl {
    answer: bool,
    asked: AtomicUsize,
}
impl UserPrompter for MockUserPrompterImpl {
    fn confirm(&self, _title: &str, _message: &str) -> Result<bool, String> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        Ok(self.answer)
    }
}

type Handler = McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl>;

fn vault(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::create_dir_all(&p).unwrap();
    p.canonicalize().unwrap()
}

fn handler(
    here: &Path,
    vaults: Vec<VaultRef>,
    prompter: Option<Arc<MockUserPrompterImpl>>,
) -> Handler {
    McpToolHandler::new(
        here.to_path_buf(),
        NoteRepositoryImpl::new(),
        None::<Arc<dyn FullTextIndex>>,
        None::<Arc<VersionControlImpl>>,
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    )
    .with_vaults(
        Some(Arc::new(InMemoryVaultCatalogImpl(vaults))),
        prompter.map(|p| p as Arc<dyn UserPrompter>),
    )
}

fn call(h: &Handler, name: &str, args: Value) -> Value {
    serde_json::to_value(h.call_tool(name, Some(args))).unwrap()
}

#[test]
fn lee_otra_boveda_con_permiso_preguntado_una_vez() {
    let tmp = tempfile::tempdir().unwrap();
    let diagramas = vault(tmp.path(), "diagramas");
    let mainframe = vault(tmp.path(), "mainframe");
    std::fs::write(
        mainframe.join("copybook.md"),
        "# Copybook TRX01\nCampos de la transferencia.\n",
    )
    .unwrap();
    let vaults = vec![
        VaultRef {
            name: "diagramas".into(),
            path: diagramas.clone(),
            open: true,
        },
        VaultRef {
            name: "mainframe".into(),
            path: mainframe.clone(),
            open: false,
        },
    ];
    let prompter = Arc::new(MockUserPrompterImpl {
        answer: true,
        asked: AtomicUsize::new(0),
    });
    let h = handler(&diagramas, vaults, Some(prompter.clone()));

    let list = call(&h, "list_vaults", json!({}));
    let list: Value = serde_json::from_str(list["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(list[0]["is_this"], true);
    assert_eq!(list[1]["name"], "mainframe");

    let read = call(
        &h,
        "read_note",
        json!({"vault": "Mainframe", "relative_path": "copybook.md"}),
    );
    assert_eq!(read["isError"], false, "{read}");
    assert!(
        read["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("📚 Bóveda «mainframe»")
    );
    assert!(
        read["content"][1]["text"]
            .as_str()
            .unwrap()
            .contains("TRX01")
    );
    // La segunda consulta usa el permiso guardado
    let again = call(
        &h,
        "read_note",
        json!({"vault": "mainframe", "relative_path": "copybook.md"}),
    );
    assert_eq!(again["isError"], false);
    assert_eq!(prompter.asked.load(Ordering::SeqCst), 1);
    assert!(diagramas.join(".synapse/vault-access.json").exists());

    // Escribir en otra bóveda no se permite
    let write = call(
        &h,
        "write_note",
        json!({"vault": "mainframe", "relative_path": "x.md", "content": "x"}),
    );
    assert_eq!(write["isError"], true);
    assert!(!mainframe.join("x.md").exists());

    // Una bóveda que no existe
    let none = call(
        &h,
        "read_note",
        json!({"vault": "nada", "relative_path": "a.md"}),
    );
    assert!(
        none["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Disponibles")
    );
}

#[test]
fn sin_permiso_o_sin_ventana_no_lee() {
    let tmp = tempfile::tempdir().unwrap();
    let a = vault(tmp.path(), "a");
    let b = vault(tmp.path(), "b");
    std::fs::write(b.join("n.md"), "# N\n").unwrap();
    let vaults = vec![
        VaultRef {
            name: "a".into(),
            path: a.clone(),
            open: true,
        },
        VaultRef {
            name: "b".into(),
            path: b.clone(),
            open: true,
        },
    ];
    let denied = Arc::new(MockUserPrompterImpl {
        answer: false,
        asked: AtomicUsize::new(0),
    });
    let h = handler(&a, vaults.clone(), Some(denied));
    let r = call(
        &h,
        "read_note",
        json!({"vault": "b", "relative_path": "n.md"}),
    );
    assert_eq!(r["isError"], true);

    // Sin ventana (modo --mcp) no se puede preguntar: se rechaza lo no autorizado
    let tmp2 = tempfile::tempdir().unwrap();
    let c = vault(tmp2.path(), "c");
    let vaults2 = vec![
        VaultRef {
            name: "c".into(),
            path: c.clone(),
            open: true,
        },
        VaultRef {
            name: "b".into(),
            path: b.clone(),
            open: true,
        },
    ];
    let h2 = handler(&c, vaults2, None);
    let r2 = call(
        &h2,
        "read_note",
        json!({"vault": "b", "relative_path": "n.md"}),
    );
    assert_eq!(r2["isError"], true);
    assert!(
        r2["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("permiso")
    );
}

#[test]
fn el_indice_de_otra_boveda_se_abre_en_solo_lectura_aunque_este_en_uso() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fts");
    // La instancia de esa bóveda lo tiene abierto (con el bloqueo de escritura)
    let owner = FullTextIndexImpl::open_or_create(&dir).unwrap();
    let reader = FullTextIndexImpl::open_read_only(&dir).unwrap();
    assert!(reader.delete_all().is_err());
    drop(owner);
    assert!(FullTextIndexImpl::open_read_only(&tmp.path().join("no-existe")).is_err());
}
