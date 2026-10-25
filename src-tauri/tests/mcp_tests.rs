use app_lib::application::use_cases::mcp_use_cases::McpUseCases;
use app_lib::infrastructure::mcp::server::McpStdioServer;
use app_lib::infrastructure::repositories::file_note_repository::FileNoteRepository;
use serde_json::json;
use std::fs;
use std::io::Cursor;
use tempfile::tempdir;

#[test]
fn test_mcp_server_initialize_tools_and_read_write_cycle() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();

    // Crear una nota de prueba
    let note1_path = vault_path.join("arquitectura.md");
    fs::write(
        &note1_path,
        "---\ntitle: Arquitectura Synapse\ntags: [rust, tauri, onion]\n---\n# Arquitectura\nDetalles del backend.",
    )
    .unwrap();

    let note_repo = FileNoteRepository::new();
    let use_cases = McpUseCases::new(vault_path.clone(), note_repo);
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

    // 2. Test tools/list
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
    assert!(tools.iter().any(|t| t["name"] == "read_note"));
    assert!(tools.iter().any(|t| t["name"] == "write_note"));

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

    // 4. Test tools/call write_note
    let write_req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "write_note",
            "arguments": {
                "relative_path": "notas/agentes.md",
                "title": "Agentes y MCP",
                "content": "# Agentes\nProtocolo MCP implementado.",
                "tags": ["mcp", "ai"]
            }
        }
    })
    .to_string();

    let resp_write = server.handle_line(&write_req).expect("Debe responder a write_note");
    let write_val: serde_json::Value = serde_json::from_str(&resp_write).unwrap();
    assert!(!write_val["result"]["isError"].as_bool().unwrap_or(false));

    let created_file = vault_path.join("notas/agentes.md");
    assert!(created_file.exists());
    let file_content = fs::read_to_string(&created_file).unwrap();
    assert!(file_content.contains("Agentes y MCP"));
    assert!(file_content.contains("mcp"));

    // 5. Test resources/list y resources/read
    let res_list_req = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "resources/list",
        "params": {}
    })
    .to_string();

    let resp_res = server.handle_line(&res_list_req).expect("Debe responder a resources/list");
    let res_val: serde_json::Value = serde_json::from_str(&resp_res).unwrap();
    let resources = res_val["result"]["resources"].as_array().unwrap();
    assert!(resources.iter().any(|r| r["uri"] == "synapse://vault/arquitectura.md"));

    // 6. Test run_loop con simulación de stream
    let input_lines = format!("{}\n{}\n", init_req, list_req);
    let mut output = Vec::new();
    let input_cursor = Cursor::new(input_lines.into_bytes());

    server.run_loop(input_cursor, &mut output).unwrap();
    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains("synapse-mcp-server"));
    assert!(output_str.contains("search_notes"));
}
