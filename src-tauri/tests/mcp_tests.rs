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

    // 6. Test run_loop con simulación de stream
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
