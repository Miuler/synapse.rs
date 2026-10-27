use app_lib::application::use_cases::mcp_use_cases::McpUseCases;
use app_lib::infrastructure::mcp::server::McpStdioServer;
use app_lib::infrastructure::repositories::file_note_repository::FileNoteRepository;
use app_lib::infrastructure::search::tantivy_index::TantivyFullTextIndex;
use app_lib::infrastructure::services::git_service::GitService;
use serde_json::json;
use std::fs;
use std::io::Cursor;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_mcp_server_initialize_tools_and_read_write_cycle() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();

    // Crear notas de prueba
    let note1_path = vault_path.join("arquitectura.md");
    fs::write(
        &note1_path,
        "---\ntitle: Arquitectura Synapse\ntags: [rust, tauri, onion]\n---\n# Arquitectura\nDetalles del backend. Ver [[agentes]].",
    )
    .unwrap();

    let note2_path = vault_path.join("agentes.md");
    fs::write(
        &note2_path,
        "# Agentes\nProtocolo MCP implementado.",
    )
    .unwrap();

    let note_repo = FileNoteRepository::new();

    // Índice Tantivy en memoria / carpeta de prueba
    let fts_dir = vault_path.join(".synapse").join("index");
    let full_text_index = TantivyFullTextIndex::open_or_create(&fts_dir).ok().map(Arc::new);
    let git_service = Some(Arc::new(GitService));

    let use_cases = McpUseCases::new(vault_path.clone(), note_repo, full_text_index, git_service);
    let server = McpStdioServer::new(use_cases);

    // 1. Test initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    })
    .to_string();

    let resp_init = server.handle_line(&init_req).expect("Debe responder a initialize");
    let init_val: serde_json::Value = serde_json::from_str(&resp_init).unwrap();
    assert_eq!(init_val["result"]["serverInfo"]["name"], "synapse-mcp-server");

    // 2. Test tools/list con todas las herramientas enriquecidas
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    })
    .to_string();

    let resp_tools = server.handle_line(&list_req).expect("Debe responder a tools/list");
    let tools_val: serde_json::Value = serde_json::from_str(&resp_tools).unwrap();
    let tools = tools_val["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "search_notes"));
    assert!(tools.iter().any(|t| t["name"] == "full_text_search"));
    assert!(tools.iter().any(|t| t["name"] == "read_note"));
    assert!(tools.iter().any(|t| t["name"] == "write_note"));
    assert!(tools.iter().any(|t| t["name"] == "get_backlinks_and_relations"));
    assert!(tools.iter().any(|t| t["name"] == "get_vault_git_status"));
    assert!(tools.iter().any(|t| t["name"] == "list_vault_tree"));

    // Validar anotaciones normativas según especificación oficial MCP
    let write_tool = tools.iter().find(|t| t["name"] == "write_note").unwrap();
    assert_eq!(write_tool["annotations"]["readOnlyHint"], false);
    assert_eq!(write_tool["annotations"]["destructiveHint"], true);
    assert_eq!(write_tool["annotations"]["idempotentHint"], true);
    assert_eq!(write_tool["annotations"]["openWorldHint"], false);

    let patch_tool = tools.iter().find(|t| t["name"] == "patch_note").unwrap();
    assert_eq!(patch_tool["annotations"]["readOnlyHint"], false);
    assert_eq!(patch_tool["annotations"]["destructiveHint"], true);
    assert_eq!(patch_tool["annotations"]["idempotentHint"], false);
    assert_eq!(patch_tool["annotations"]["openWorldHint"], false);

    let append_tool = tools.iter().find(|t| t["name"] == "append_to_note").unwrap();
    assert_eq!(append_tool["annotations"]["readOnlyHint"], false);
    assert_eq!(append_tool["annotations"]["destructiveHint"], false);
    assert_eq!(append_tool["annotations"]["idempotentHint"], false);
    assert_eq!(append_tool["annotations"]["openWorldHint"], false);

    let move_tool = tools.iter().find(|t| t["name"] == "move_notes").unwrap();
    assert_eq!(move_tool["annotations"]["readOnlyHint"], false);
    assert_eq!(move_tool["annotations"]["destructiveHint"], true);
    assert_eq!(move_tool["annotations"]["idempotentHint"], false);
    assert_eq!(move_tool["annotations"]["openWorldHint"], false);

    let delete_tool = tools.iter().find(|t| t["name"] == "delete_note").unwrap();
    assert_eq!(delete_tool["annotations"]["readOnlyHint"], false);
    assert_eq!(delete_tool["annotations"]["destructiveHint"], true);
    assert_eq!(delete_tool["annotations"]["idempotentHint"], false);
    assert_eq!(delete_tool["annotations"]["openWorldHint"], false);

    let read_tool = tools.iter().find(|t| t["name"] == "read_note").unwrap();
    assert_eq!(read_tool["annotations"]["readOnlyHint"], true);
    assert_eq!(read_tool["annotations"]["destructiveHint"], false);
    assert_eq!(read_tool["annotations"]["idempotentHint"], true);
    assert_eq!(read_tool["annotations"]["openWorldHint"], false);

    // 3. Test tools/call search_notes
    let search_req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "search_notes",
            "arguments": {
                "query": "arquitectura"
            }
        }
    })
    .to_string();

    let resp_search = server.handle_line(&search_req).expect("Debe responder a search_notes");
    let search_val: serde_json::Value = serde_json::from_str(&resp_search).unwrap();
    let text = search_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("arquitectura.md"));
    assert!(text.contains("Arquitectura Synapse"));

    // 4. Test tools/call get_backlinks_and_relations para 'agentes.md'
    let backlinks_req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "get_backlinks_and_relations",
            "arguments": {
                "relative_path": "agentes.md"
            }
        }
    })
    .to_string();

    let resp_bl = server.handle_line(&backlinks_req).expect("Debe responder a get_backlinks_and_relations");
    let bl_val: serde_json::Value = serde_json::from_str(&resp_bl).unwrap();
    let bl_text = bl_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(bl_text.contains("arquitectura.md")); // arquitectura.md tiene un enlace hacia [[agentes]]

    // 5. Test tools/call write_note
    let write_req = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "write_note",
            "arguments": {
                "relative_path": "notas/nuevo.md",
                "title": "Nuevo Documento",
                "content": "# Nuevo\nContenido creado por agente.",
                "tags": ["ia", "mcp"]
            }
        }
    })
    .to_string();

    let resp_write = server.handle_line(&write_req).expect("Debe responder a write_note");
    let write_val: serde_json::Value = serde_json::from_str(&resp_write).unwrap();
    assert!(!write_val["result"]["isError"].as_bool().unwrap_or(false));

    // Crear segunda nota en la misma carpeta 'notas/' y una subcarpeta 'notas/sub'
    let write_req_2 = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "write_note",
            "arguments": {
                "relative_path": "notas/hermana.md",
                "title": "Nota Hermana",
                "content": "# Hermana\nOtra nota en la misma carpeta.",
                "tags": ["hermana"]
            }
        }
    })
    .to_string();
    let _ = server.handle_line(&write_req_2).expect("Debe guardar hermana.md");
    std::fs::create_dir_all(vault_path.join("notas/subcarpeta")).unwrap();

    // 6. Test list_folder_contents sobre 'notas'
    let folder_req = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "list_folder_contents",
            "arguments": {
                "folder_path": "notas",
                "include_notes": true,
                "include_folders": true
            }
        }
    })
    .to_string();
    let resp_folder = server.handle_line(&folder_req).expect("Debe responder a list_folder_contents");
    let folder_val: serde_json::Value = serde_json::from_str(&resp_folder).unwrap();
    let folder_text = folder_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(folder_text.contains("notas/nuevo.md"));
    assert!(folder_text.contains("notas/hermana.md"));
    assert!(folder_text.contains("notas/subcarpeta"));

    // 7. Test list_sibling_notes a partir de 'notas/nuevo.md'
    let siblings_req = json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "tools/call",
        "params": {
            "name": "list_sibling_notes",
            "arguments": {
                "relative_path": "notas/nuevo.md",
                "include_self": false,
                "include_folders": false
            }
        }
    })
    .to_string();
    let resp_siblings = server.handle_line(&siblings_req).expect("Debe responder a list_sibling_notes");
    let siblings_val: serde_json::Value = serde_json::from_str(&resp_siblings).unwrap();
    let siblings_text = siblings_val["result"]["content"][0]["text"].as_str().unwrap();
    let parsed_siblings: serde_json::Value = serde_json::from_str(siblings_text).unwrap();
    assert_eq!(parsed_siblings["notes_count"], 1);
    assert_eq!(parsed_siblings["notes"][0]["relative_path"], "notas/hermana.md");
    assert_eq!(parsed_siblings["notes"][0]["title"], "Nota Hermana");

    // 8. Test move_notes moviendo lista de notas a una subcarpeta destino
    let move_req = json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "tools/call",
        "params": {
            "name": "move_notes",
            "arguments": {
                "notes": ["notas/nuevo.md", "notas/hermana.md"],
                "target_folder": "archivo/2026",
                "create_target_folder": true
            }
        }
    })
    .to_string();
    let resp_move = server.handle_line(&move_req).expect("Debe responder a move_notes");
    let move_val: serde_json::Value = serde_json::from_str(&resp_move).unwrap();
    let move_text = move_val["result"]["content"][0]["text"].as_str().unwrap();
    let parsed_move: serde_json::Value = serde_json::from_str(move_text).unwrap();
    assert_eq!(parsed_move["moved_count"], 2);
    assert_eq!(parsed_move["errors_count"], 0);
    assert!(vault_path.join("archivo/2026/nuevo.md").exists());
    assert!(vault_path.join("archivo/2026/hermana.md").exists());
    assert!(!vault_path.join("notas/nuevo.md").exists());

    // 9. Test delete_note (incluyendo verificación de expected_hash y eliminación física)
    let del_occ_mismatch_req = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "delete_note",
            "arguments": {
                "relative_path": "archivo/2026/nuevo.md",
                "expected_hash": "sha256:hashincorrecto12345"
            }
        }
    })
    .to_string();
    let resp_del_fail = server.handle_line(&del_occ_mismatch_req).expect("Debe responder a delete_note");
    let del_fail_val: serde_json::Value = serde_json::from_str(&resp_del_fail).unwrap();
    assert!(del_fail_val["result"]["isError"].as_bool().unwrap_or(false));
    assert!(vault_path.join("archivo/2026/nuevo.md").exists());

    let del_req = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "delete_note",
            "arguments": {
                "relative_path": "archivo/2026/nuevo.md"
            }
        }
    })
    .to_string();
    let resp_del = server.handle_line(&del_req).expect("Debe eliminar la nota");
    let del_val: serde_json::Value = serde_json::from_str(&resp_del).unwrap();
    assert!(!del_val["result"]["isError"].as_bool().unwrap_or(false));
    assert!(!vault_path.join("archivo/2026/nuevo.md").exists());

    // 10. Test run_loop con simulación de stream
    let input_lines = format!("{}\n{}\n", init_req, list_req);
    let mut output = Vec::new();
    let input_cursor = Cursor::new(input_lines.into_bytes());

    server.run_loop(input_cursor, &mut output).unwrap();
    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains("synapse-mcp-server"));
    assert!(output_str.contains("full_text_search"));
}

struct DummyMcpProvider {
    vault: std::path::PathBuf,
}

impl app_lib::infrastructure::mcp::server::McpContextProvider for DummyMcpProvider {
    fn get_context(&self) -> (Option<std::path::PathBuf>, Option<std::sync::Arc<dyn app_lib::domain::services::full_text_index::FullTextIndex>>) {
        (Some(self.vault.clone()), None)
    }
}

#[test]
fn test_mcp_background_runtime_lifecycle_and_http() {
    use app_lib::infrastructure::mcp::server::McpBackgroundRuntime;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::Arc;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let vault_path = temp_dir.path().to_path_buf();
    let provider = Arc::new(DummyMcpProvider { vault: vault_path });
    let runtime = McpBackgroundRuntime::new(provider);

    // 1. Estado inicial
    let initial_status = runtime.get_status();
    assert!(!initial_status.running);

    // 2. Start
    let started_status = runtime.start().expect("Debe iniciar runtime MCP");
    assert!(started_status.running);
    let port = started_status.port;

    // 3. Probar GET /health
    {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("Debe conectar a servidor HTTP");
        stream.write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("synapse-mcp-server"));
    }

    // 4. Probar POST /mcp con JSON-RPC initialize
    {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "initialize",
            "params": {}
        }).to_string();

        let req = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            body
        );

        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("Debe conectar a servidor HTTP");
        stream.write_all(req.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.contains("protocolVersion"));
        assert!(response.contains("synapse-mcp-server"));
    }

    // 5. Toggle (apagar)
    let toggled_status = runtime.toggle().expect("Debe alternar runtime");
    assert!(!toggled_status.running);
    assert_eq!(toggled_status.requests_handled, 2);
}

#[test]
fn test_mcp_evolved_tools_suite() {
    use app_lib::application::use_cases::mcp_use_cases::McpUseCases;
    use app_lib::domain::models::note::Note;
    use app_lib::domain::repositories::note_repository::NoteRepository;
    use app_lib::domain::value_objects::note_path::NoteRelativePath;
    use app_lib::infrastructure::mcp::server::McpStdioServer;
    use app_lib::infrastructure::repositories::file_note_repository::FileNoteRepository;
    use app_lib::infrastructure::services::git_service::GitService;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let vault_path = temp_dir.path().to_path_buf();
    let repo = FileNoteRepository::new();
    let git = Arc::new(GitService::new());

    // Crear nota de prueba con múltiples secciones, frontmatter y bloques especiales
    let initial_content = r#"---
title: Arquitectura Synapse
tags: [rust, mcp, acp]
aliases: [Synapse Arch]
---

# Introducción
Este es el párrafo inicial de arquitectura.

## Componentes
- Frontend: Svelte 5 Runes
- Backend: Rust Tauri v2

```mermaid
graph TD
A[Cliente ACP] --> B[Servidor MCP]
```

## Tareas
- [ ] Implementar patch_note
- [x] Diseñar especificación
"#;

    let rel_path = NoteRelativePath::new("arquitectura.md").unwrap();
    let note = Note::with_encoding(rel_path.clone(), "".to_string(), "Arquitectura Synapse".to_string(), initial_content.to_string(), "UTF-8".to_string());
    repo.save_note(&vault_path, &note).unwrap();

    let use_cases = McpUseCases::new(
        vault_path.clone(),
        FileNoteRepository::new(),
        None::<Arc<dyn app_lib::domain::services::full_text_index::FullTextIndex>>,
        Some(git),
    );
    let server = McpStdioServer::new(use_cases);

    // 1. Probar read_note en mode='metadata_only'
    let req_meta = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "read_note",
            "arguments": {
                "relative_path": "arquitectura.md",
                "mode": "metadata_only"
            }
        }
    }).to_string();
    let resp_meta = server.handle_line(&req_meta).unwrap();
    let val_meta: serde_json::Value = serde_json::from_str(&resp_meta).unwrap();
    let meta_obj: serde_json::Value = serde_json::from_str(val_meta["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(meta_obj["content_hash"].as_str().unwrap().starts_with("sha256:"));
    assert_eq!(meta_obj["detected_blocks"]["mermaid_count"], 1);
    assert_eq!(meta_obj["toc"].as_array().unwrap().len(), 3);
    let expected_hash = meta_obj["content_hash"].as_str().unwrap().to_string();

    // 2. Probar search_in_note
    let req_search_in = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "search_in_note",
            "arguments": {
                "relative_path": "arquitectura.md",
                "query": "Svelte 5"
            }
        }
    }).to_string();
    let resp_search_in = server.handle_line(&req_search_in).unwrap();
    let val_search_in: serde_json::Value = serde_json::from_str(&resp_search_in).unwrap();
    let search_in_obj: serde_json::Value = serde_json::from_str(val_search_in["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(search_in_obj["total_matches"], 1);
    assert!(search_in_obj["matches"][0]["context"].as_array().unwrap().len() >= 1);

    // 3. Probar patch_note con dry_run y luego aplicación real con expected_hash
    let req_patch_dry = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "patch_note",
            "arguments": {
                "relative_path": "arquitectura.md",
                "old_text": "Backend: Rust Tauri v2",
                "new_text": "Backend: Rust Tauri v2 con Onion Architecture",
                "expected_hash": expected_hash,
                "dry_run": true
            }
        }
    }).to_string();
    let resp_patch_dry = server.handle_line(&req_patch_dry).unwrap();
    let val_patch_dry: serde_json::Value = serde_json::from_str(&resp_patch_dry).unwrap();
    let patch_dry_obj: serde_json::Value = serde_json::from_str(val_patch_dry["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(patch_dry_obj["unified_diff"].as_str().unwrap().contains("+- Backend: Rust Tauri v2 con Onion Architecture"));

    // Aplicar patch real
    let req_patch_real = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "patch_note",
            "arguments": {
                "relative_path": "arquitectura.md",
                "old_text": "Backend: Rust Tauri v2",
                "new_text": "Backend: Rust Tauri v2 con Onion Architecture",
                "expected_hash": expected_hash
            }
        }
    }).to_string();
    let resp_patch_real = server.handle_line(&req_patch_real).unwrap();
    let val_patch_real: serde_json::Value = serde_json::from_str(&resp_patch_real).unwrap();
    let patch_real_obj: serde_json::Value = serde_json::from_str(val_patch_real["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(patch_real_obj["status"], "success");

    // Verificar en disco
    let updated_read = repo.read_note(&vault_path, &rel_path).unwrap();
    assert!(updated_read.content.contains("Backend: Rust Tauri v2 con Onion Architecture"));

    // 4. Probar OCC failure en patch_note (hash anterior ya no es válido)
    let req_patch_occ_fail = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "patch_note",
            "arguments": {
                "relative_path": "arquitectura.md",
                "old_text": "Frontend: Svelte 5 Runes",
                "new_text": "Frontend: Svelte 5 Runes + FSD",
                "expected_hash": expected_hash
            }
        }
    }).to_string();
    let resp_patch_occ_fail = server.handle_line(&req_patch_occ_fail).unwrap();
    assert!(resp_patch_occ_fail.contains("Conflicto OCC") || resp_patch_occ_fail.contains("isError"));

    // 5. Probar append_to_note dentro de sección Tareas
    let req_append = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "append_to_note",
            "arguments": {
                "relative_path": "arquitectura.md",
                "content": "- [ ] Tarea agregada automáticamente por agente MCP",
                "section_heading": "## Tareas"
            }
        }
    }).to_string();
    let resp_append = server.handle_line(&req_append).unwrap();
    assert!(!resp_append.contains(r#""isError":true"#));
    let after_append = repo.read_note(&vault_path, &rel_path).unwrap();
    assert!(after_append.content.contains("- [ ] Tarea agregada automáticamente por agente MCP"));

    // 6. Probar update_note_metadata atómico
    let req_meta_up = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "update_note_metadata",
            "arguments": {
                "relative_path": "arquitectura.md",
                "add_tags": ["onion-architecture", "tested"],
                "remove_tags": ["acp"]
            }
        }
    }).to_string();
    let resp_meta_up = server.handle_line(&req_meta_up).unwrap();
    assert!(!resp_meta_up.contains(r#""isError":true"#));
    let after_meta = repo.read_note(&vault_path, &rel_path).unwrap();
    let tags = after_meta.frontmatter.unwrap().tags;
    assert!(tags.contains(&"onion-architecture".to_string()));
    assert!(tags.contains(&"tested".to_string()));
    assert!(!tags.contains(&"acp".to_string()));

    // 7. Probar extract_note_diagrams
    let req_diagrams = json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "tools/call",
        "params": {
            "name": "extract_note_diagrams",
            "arguments": {
                "relative_path": "arquitectura.md"
            }
        }
    }).to_string();
    let resp_diagrams = server.handle_line(&req_diagrams).unwrap();
    let val_diag: serde_json::Value = serde_json::from_str(&resp_diagrams).unwrap();
    let diag_obj: serde_json::Value = serde_json::from_str(val_diag["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(diag_obj["total_diagrams"], 1);
    assert_eq!(diag_obj["diagrams"][0]["type"], "mermaid");
    assert!(diag_obj["diagrams"][0]["code"].as_str().unwrap().contains("graph TD"));
}
