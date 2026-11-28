//! La carpeta de un tablero (`Nombre.board/`) no se puede cambiar por ninguna vía de Synapse que no
//! sea el propio tablero, y acompaña a su `.board.md` cuando este se renombra, mueve o copia.

use app_lib::domain::note::note::Note;
use app_lib::domain::note::note_path::NoteRelativePath;
use app_lib::domain::note::note_repository::NoteRepository;
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::domain::vault::vault_item_repository::VaultItemRepository;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::sync::Arc;

const TASK: &str = "# Tarea\n\n- status: todo\n";

fn vault() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("Equipo.board")).unwrap();
    fs::create_dir_all(root.join("destino")).unwrap();
    fs::write(
        root.join("Equipo.board.md"),
        "---\ntype: board\nformat: 2\n---\n",
    )
    .unwrap();
    fs::write(root.join("Equipo.board/t1.md"), TASK).unwrap();
    fs::write(root.join("nota.md"), "# Nota\n").unwrap();
    dir
}

/// La carpeta sigue igual: la misma tarea y nada más.
fn intact(root: &Path, how: &str) {
    let names: Vec<String> = fs::read_dir(root.join("Equipo.board"))
        .unwrap_or_else(|_| panic!("{how}: la carpeta ya no está"))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["t1.md"], "{how}");
    assert_eq!(
        fs::read_to_string(root.join("Equipo.board/t1.md")).unwrap(),
        TASK,
        "{how}"
    );
}

#[test]
fn las_notas_y_los_elementos_no_escriben_en_la_carpeta() {
    let dir = vault();
    let root = dir.path();
    let items = VaultItemRepositoryImpl;
    let notes = NoteRepositoryImpl::new();
    let task = "Equipo.board/t1.md";
    let path = NoteRelativePath::new(task).unwrap();
    let note = Note::new(path.clone(), String::new(), "T".into(), "cambiado".into());
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("fuera.md"), "x").unwrap();

    let attempts: Vec<(&str, Result<(), String>)> = vec![
        ("guardar", notes.save_note(root, &note)),
        (
            "guardar suelto",
            notes.save_standalone_file(&root.join(task), &note),
        ),
        ("borrar nota", notes.delete_note(root, &path)),
        ("borrar", items.delete_item(root, task)),
        ("borrar la carpeta", items.delete_item(root, "Equipo.board")),
        (
            "renombrar",
            items.rename_item(root, task, "t2.md").map(drop),
        ),
        (
            "renombrar la carpeta",
            items.rename_item(root, "Equipo.board", "Otra").map(drop),
        ),
        (
            "mover fuera",
            items.move_item(root, task, "destino").map(drop),
        ),
        (
            "mover dentro",
            items.move_item(root, "nota.md", "Equipo.board").map(drop),
        ),
        (
            "copiar dentro",
            items.copy_item(root, "nota.md", "Equipo.board").map(drop),
        ),
        (
            "duplicar",
            items.copy_item(root, task, "Equipo.board").map(drop),
        ),
        (
            "importar",
            items
                .import_item(root, &outside.path().join("fuera.md"), "Equipo.board")
                .map(drop),
        ),
        (
            "binario",
            items.write_binary_item(root, "Equipo.board/x.png", b"x"),
        ),
        (
            "crear carpeta",
            items.create_folder(root, "Equipo.board/sub"),
        ),
        (
            "asegurar carpeta",
            items.ensure_folder(root, "Equipo.board/sub"),
        ),
        ("reubicar dentro", items.relocate(root, "nota.md", task)),
        (
            "reubicar fuera",
            items.relocate(root, task, "destino/t1.md"),
        ),
    ];
    for (how, result) in attempts {
        let error = result.expect_err(how);
        assert!(
            error.contains("(Equipo.board.md): belongs to that board"),
            "{how}: {error}"
        );
        intact(root, how);
    }
    assert!(root.join("nota.md").exists());
    // Leer y copiar hacia fuera sí se puede
    assert_eq!(items.read_text(root, task).unwrap(), TASK);
    items.copy_item(root, task, "destino").unwrap();
}

#[test]
fn las_herramientas_mcp_tampoco() {
    let dir = vault();
    let root = dir.path();
    let handler: McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl> =
        McpToolHandler::new(
            root.to_path_buf(),
            NoteRepositoryImpl::new(),
            None::<Arc<dyn FullTextIndex>>,
            None::<Arc<VersionControlImpl>>,
            Arc::new(VaultItemRepositoryImpl),
            Arc::new(PdfDocumentRepositoryImpl),
        );
    let task = "Equipo.board/t1.md";
    let calls: Vec<(&str, Value)> = vec![
        ("write_note", json!({"relative_path": task, "content": "x", "overwrite": true})),
        (
            "write_note",
            json!({"relative_path": "Equipo.board/t2.md", "content": "x"}),
        ),
        (
            "patch_note",
            json!({"relative_path": task, "old_text": "todo", "new_text": "done"}),
        ),
        (
            "append_to_note",
            json!({"relative_path": task, "content": "x"}),
        ),
        (
            "update_note_metadata",
            json!({"relative_path": task, "title": "x"}),
        ),
        ("delete_note", json!({"relative_path": task})),
        (
            "move_notes",
            json!({"notes": [task], "target_folder": "destino"}),
        ),
        (
            "move_notes",
            json!({"notes": ["nota.md"], "target_folder": "Equipo.board"}),
        ),
        (
            "move_folder_contents",
            json!({"source_folder": "Equipo.board", "target_folder": "destino"}),
        ),
    ];
    for (tool, args) in calls {
        let how = format!("{tool} {args}");
        let answer = serde_json::to_string(&handler.call_tool(tool, Some(args))).unwrap();
        assert!(answer.contains("belongs to that board"), "{how}: {answer}");
        intact(root, &how);
    }
    assert!(root.join("nota.md").exists());
}

#[test]
fn la_carpeta_acompana_a_su_tablero() {
    let dir = vault();
    let root = dir.path();
    let items = VaultItemRepositoryImpl;
    let task = |path: &str| fs::read_to_string(root.join(path)).unwrap();

    assert!(
        items
            .rename_item(root, "Equipo.board.md", "Equipo.md")
            .is_err()
    );
    fs::create_dir_all(root.join("Ocupado.board")).unwrap();
    assert!(
        items
            .rename_item(root, "Equipo.board.md", "Ocupado.board.md")
            .is_err()
    );
    intact(root, "renombrados rechazados");

    items
        .rename_item(root, "Equipo.board.md", "Soporte.board.md")
        .unwrap();
    assert_eq!(task("Soporte.board/t1.md"), TASK);
    assert!(!root.join("Equipo.board").exists());

    items
        .move_item(root, "Soporte.board.md", "destino")
        .unwrap();
    assert_eq!(task("destino/Soporte.board/t1.md"), TASK);

    let copy = items
        .copy_item(root, "destino/Soporte.board.md", "destino")
        .unwrap();
    let folder = copy.strip_suffix(".md").unwrap();
    assert!(folder.ends_with(".board"), "{copy}");
    assert_eq!(task(&format!("{folder}/t1.md")), TASK);

    // Mover la carpeta que los contiene se lleva los dos
    items.create_folder(root, "archivo").unwrap();
    items.move_item(root, "destino", "archivo").unwrap();
    assert_eq!(task("archivo/destino/Soporte.board/t1.md"), TASK);

    // Sin su tablero, la carpeta queda libre y se puede borrar
    items
        .delete_item(root, "archivo/destino/Soporte.board.md")
        .unwrap();
    items
        .delete_item(root, "archivo/destino/Soporte.board")
        .unwrap();
}
