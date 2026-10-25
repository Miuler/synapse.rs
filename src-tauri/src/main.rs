// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_lib::application::use_cases::mcp_use_cases::McpUseCases;
use app_lib::infrastructure::mcp::server::McpStdioServer;
use app_lib::infrastructure::repositories::file_note_repository::FileNoteRepository;
use app_lib::infrastructure::search::tantivy_index::TantivyFullTextIndex;
use app_lib::infrastructure::services::git_service::GitService;
use std::env;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Comprobar si se solicitó arrancar como servidor MCP
    let is_mcp_mode = args.iter().any(|arg| arg == "--mcp");

    if is_mcp_mode {
        // En modo MCP, todos los logs deben ir a stderr para no contaminar stdout (canal JSON-RPC)
        eprintln!("[Synapse MCP] Iniciando servidor MCP local vía stdio...");

        // Determinar ruta de la bóveda
        let mut vault_path = None;
        let mut iter = args.iter().skip(1);
        while let Some(arg) = iter.next() {
            if arg == "--vault" || arg == "-v" {
                if let Some(target) = iter.next() {
                    vault_path = Some(PathBuf::from(target));
                }
            }
        }

        let resolved_vault = vault_path.unwrap_or_else(|| {
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });

        let canonical_vault = resolved_vault.canonicalize().unwrap_or(resolved_vault);
        eprintln!("[Synapse MCP] Bóveda activa: {:?}", canonical_vault);

        let note_repo = FileNoteRepository::new();

        // Inicializar Tantivy Index si existe el directorio .synapse/index o abrir/crear
        let fts_dir = canonical_vault.join(".synapse").join("index");
        let full_text_index = match TantivyFullTextIndex::open_or_create(&fts_dir) {
            Ok(idx) => Some(Arc::new(idx)),
            Err(e) => {
                eprintln!("[Synapse MCP] Advertencia: No se pudo abrir índice Tantivy: {}", e);
                None
            }
        };

        // Inicializar GitService
        let git_service = Some(Arc::new(GitService));

        let mcp_use_cases = McpUseCases::new(canonical_vault, note_repo, full_text_index, git_service);
        let mcp_server = McpStdioServer::new(mcp_use_cases);

        let stdin = io::stdin();
        let stdout = io::stdout();

        if let Err(e) = mcp_server.run_loop(stdin.lock(), stdout.lock()) {
            eprintln!("[Synapse MCP] Error en el bucle stdio: {}", e);
        }
        return;
    }

    // Modo normal: aplicación de escritorio Tauri
    app_lib::run();
}
