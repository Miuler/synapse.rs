use app_lib::domain::events::vault_events::{VaultChange, VaultChangeObserver};
use app_lib::domain::models::file_types::SupportedFileTypes;
use app_lib::domain::models::full_text::{DocumentKind, FullTextQuery};
use app_lib::domain::services::full_text_index::FullTextIndex;
use app_lib::infrastructure::search::fts_indexer::FtsIndexer;
use app_lib::infrastructure::search::tantivy_index::TantivyFullTextIndex;
use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_full_text_indexer_workflow_and_persistence() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().to_path_buf();
    let fts_dir = vault_path.join(".synapse").join("fts");
    let file_types = SupportedFileTypes::default();

    // 1. Crear notas iniciales en el vault
    let note1_path = vault_path.join("arquitectura.md");
    fs::write(
        &note1_path,
        "# Arquitectura Onion\n\nSynapse utiliza arquitectura cebolla con Rust y Tantivy.",
    )
    .unwrap();

    let note2_path = vault_path.join("musica.md");
    fs::write(
        &note2_path,
        "# Cancionero\n\nColección de hermosas canciones acústicas.",
    )
    .unwrap();

    let sub_dir = vault_path.join("src");
    fs::create_dir_all(&sub_dir).unwrap();
    let code_path = sub_dir.join("main.rs");
    fs::write(
        &code_path,
        "fn main() {\n    println!(\"Super motor de búsqueda full-text\");\n}",
    )
    .unwrap();

    // 2. Abrir índice Tantivy en disco e iniciar FtsIndexer
    let index = Arc::new(TantivyFullTextIndex::open_or_create(&fts_dir).unwrap());
    let indexer = FtsIndexer::start(vault_path.clone(), file_types.clone(), Arc::clone(&index));

    // Esperar sincronización inicial
    for _ in 0..30 {
        if index.num_docs() == 3 {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    // Comprobar que los 3 documentos están indexados
    assert_eq!(index.num_docs(), 3);

    // Búsqueda 1: buscar en el cuerpo de una nota markdown
    let q1 = FullTextQuery {
        text: "cebolla".to_string(),
        limit: 10,
        prefix_last_term: false,
    };
    let (hits1, count1) = index.search(&q1).unwrap();
    assert_eq!(count1, 1);
    assert_eq!(hits1[0].note_path, "arquitectura.md");
    assert_eq!(hits1[0].kind, DocumentKind::Markdown);
    assert!(!hits1[0].snippet.is_empty());

    // Búsqueda 2: stemming español (cancion -> canciones)
    let q2 = FullTextQuery {
        text: "cancion".to_string(),
        limit: 10,
        prefix_last_term: false,
    };
    let (hits2, count2) = index.search(&q2).unwrap();
    assert_eq!(count2, 1);
    assert_eq!(hits2[0].note_path, "musica.md");

    // Búsqueda 3: buscar en código
    let q3 = FullTextQuery {
        text: "motor".to_string(),
        limit: 10,
        prefix_last_term: false,
    };
    let (hits3, count3) = index.search(&q3).unwrap();
    assert_eq!(count3, 1);
    assert_eq!(hits3[0].note_path, "src/main.rs");
    assert_eq!(hits3[0].kind, DocumentKind::Code);

    // 3. Probar modificación reactiva a través del observer
    fs::write(
        &note1_path,
        "# Arquitectura Onion\n\nSe ha añadido microservicios y contenedores.",
    )
    .unwrap();
    indexer.on_vault_change(VaultChange::Upserted(vec!["arquitectura.md".to_string()]));

    // Esperar debounce
    thread::sleep(Duration::from_millis(600));

    let q_new = FullTextQuery {
        text: "microservicios".to_string(),
        limit: 10,
        prefix_last_term: false,
    };
    let (hits_new, count_new) = index.search(&q_new).unwrap();
    assert_eq!(count_new, 1);
    assert_eq!(hits_new[0].note_path, "arquitectura.md");

    // La palabra anterior "cebolla" ya no debería existir en nota1
    let (hits_old, count_old) = index.search(&q1).unwrap();
    assert_eq!(count_old, 0);
    assert_eq!(hits_old.len(), 0);

    // 4. Probar eliminación de un archivo a través del observer
    fs::remove_file(&note2_path).unwrap();
    indexer.on_vault_change(VaultChange::Removed(vec!["musica.md".to_string()]));
    thread::sleep(Duration::from_millis(600));

    assert_eq!(index.num_docs(), 2);
    let (_, count_deleted) = index.search(&q2).unwrap();
    assert_eq!(count_deleted, 0);

    // 5. Probar persistencia del índice tras reiniciar
    drop(indexer);
    drop(index);

    // Reabrir índice existente desde fts_dir
    let reopened_index = TantivyFullTextIndex::open_or_create(&fts_dir).unwrap();
    assert_eq!(reopened_index.num_docs(), 2);
    let versions = reopened_index.indexed_versions().unwrap();
    assert!(versions.contains_key("arquitectura.md"));
    assert!(versions.contains_key("src/main.rs"));
    assert!(!versions.contains_key("musica.md"));
}

#[test]
fn test_full_text_docx_and_pdf_indexing() {
    use lopdf::content::Content;
    use lopdf::{dictionary, Document, Object, Stream};
    use std::io::{Cursor, Write};
    use zip::write::{SimpleFileOptions, ZipWriter};

    let dir = tempdir().unwrap();
    let vault_path = dir.path().to_path_buf();
    let fts_dir = vault_path.join(".synapse").join("fts");
    let file_types = SupportedFileTypes::default();

    // 1. Crear documento Word (.docx)
    let docx_path = vault_path.join("balance_financiero.docx");
    {
        let mut buffer = Vec::new();
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();
        zip.start_file("word/document.xml", options).unwrap();
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body><w:p><w:r><w:t>Informe trimestral de ventas y balance financiero corporativo</w:t></w:r></w:p></w:body>
        </w:document>"#;
        zip.write_all(xml.as_bytes()).unwrap();
        zip.finish().unwrap();
        fs::write(&docx_path, buffer).unwrap();
    }

    // 2. Crear documento PDF (.pdf)
    let pdf_path = vault_path.join("estrategia.pdf");
    {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });

        let content = Content {
            operations: vec![
                lopdf::content::Operation::new("BT", vec![]),
                lopdf::content::Operation::new("Tf", vec!["F1".into(), 12.into()]),
                lopdf::content::Operation::new("Tj", vec![Object::string_literal("Plan estrategico de expansion multinacional")]),
                lopdf::content::Operation::new("ET", vec![]),
            ],
        };

        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
        });

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        fs::write(&pdf_path, buffer).unwrap();
    }

    // 3. Iniciar indexación con FtsIndexer
    let index = Arc::new(TantivyFullTextIndex::open_or_create(&fts_dir).unwrap());
    let indexer = FtsIndexer::start(vault_path.clone(), file_types, Arc::clone(&index));

    // Esperar sincronización de ambos archivos
    for _ in 0..40 {
        if index.num_docs() == 2 {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    assert_eq!(index.num_docs(), 2);

    // 4. Buscar en documento Word
    let q_docx = FullTextQuery {
        text: "trimestral".to_string(),
        limit: 10,
        prefix_last_term: false,
    };
    let (hits_docx, count_docx) = index.search(&q_docx).unwrap();
    assert_eq!(count_docx, 1);
    assert_eq!(hits_docx[0].note_path, "balance_financiero.docx");
    assert_eq!(hits_docx[0].kind, DocumentKind::Word);
    assert!(hits_docx[0].snippet.iter().any(|s| s.text.to_lowercase().contains("trimestral")));

    // 5. Buscar en documento PDF
    let q_pdf = FullTextQuery {
        text: "estrategico".to_string(),
        limit: 10,
        prefix_last_term: false,
    };
    let (hits_pdf, count_pdf) = index.search(&q_pdf).unwrap();
    assert_eq!(count_pdf, 1);
    assert_eq!(hits_pdf[0].note_path, "estrategia.pdf");
    assert_eq!(hits_pdf[0].kind, DocumentKind::Pdf);
    assert!(hits_pdf[0].snippet.iter().any(|s| s.text.to_lowercase().contains("estrategico")));

    drop(indexer);
}
