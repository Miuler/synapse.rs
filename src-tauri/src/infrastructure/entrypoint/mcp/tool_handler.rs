use crate::application::dto::note::append_to_note_command::AppendToNoteCommand;
use crate::infrastructure::entrypoint::mcp::config_tools::{
    self, ConfigAssistant, NOTHING_OPEN, PRIVATE_TOOL_PREFIX,
};
use crate::application::dto::note::delete_note_command::DeleteNoteCommand;
use crate::application::dto::note::extract_note_diagrams_query::{
    ExtractNoteDiagramsQuery, NoteDiagrams,
};
use crate::application::dto::note::get_note_relations_query::GetNoteRelationsQuery;
use crate::application::dto::note::list_notes_query::ListNotesQuery;
use crate::application::dto::note::patch_note_command::PatchNoteCommand;
use crate::application::dto::note::read_note_details_query::ReadNoteDetailsQuery;
use crate::application::dto::note::read_note_query::ReadNoteQuery;
use crate::application::dto::note::search_in_note_query::SearchInNoteQuery;
use crate::application::dto::note::search_notes_query::SearchNotesQuery;
use crate::application::dto::note::update_note_metadata_command::UpdateNoteMetadataCommand;
use crate::application::dto::note::write_note_command::{WriteNoteCommand, WriteNoteOutcome};
use crate::application::dto::pdf::read_pdf_document_query::{PdfReading, ReadPdfDocumentQuery};
use crate::application::dto::search::search_notes_full_text_query::SearchNotesFullTextQuery;
use crate::application::dto::vault_item::list_folder_contents_query::ListFolderContentsQuery;
use crate::application::dto::vault_item::move_folder_contents_command::{
    MoveFolderContentsCommand, MoveFolderContentsResult,
};
use crate::application::dto::vault_item::move_notes_command::MoveNotesCommand;
use crate::application::dto::vault_item::search_folders_query::SearchFoldersQuery;
use crate::application::dto::version::get_vault_version_state_query::GetVaultVersionStateQuery;
use crate::application::service::conversation_hub::ConversationHub;
use crate::application::service::diagram_renderer::{DiagramRenderer, diagram_kind};
use crate::application::service::vault_catalog::{UserPrompter, VaultCatalog, VaultRef};
use crate::application::use_cases::note::append_to_note_use_case::AppendToNoteUseCase;
use crate::application::use_cases::note::delete_note_use_case::DeleteNoteUseCase;
use crate::application::use_cases::note::extract_note_diagrams_use_case::ExtractNoteDiagramsUseCase;
use crate::application::use_cases::note::get_note_relations_use_case::GetNoteRelationsUseCase;
use crate::application::use_cases::note::list_notes_use_case::ListNotesUseCase;
use crate::application::use_cases::note::patch_note_use_case::PatchNoteUseCase;
use crate::application::use_cases::note::read_note_details_use_case::ReadNoteDetailsUseCase;
use crate::application::use_cases::note::read_note_use_case::ReadNoteUseCase;
use crate::application::use_cases::note::search_in_note_use_case::SearchInNoteUseCase;
use crate::application::use_cases::note::search_notes_use_case::SearchNotesUseCase;
use crate::application::use_cases::note::update_note_metadata_use_case::UpdateNoteMetadataUseCase;
use crate::application::use_cases::note::write_note_use_case::WriteNoteUseCase;
use crate::application::use_cases::pdf::read_pdf_document_use_case::ReadPdfDocumentUseCase;
use crate::application::use_cases::search::search_notes_full_text_use_case::SearchNotesFullTextUseCase;
use crate::application::use_cases::vault_item::list_folder_contents_use_case::ListFolderContentsUseCase;
use crate::application::use_cases::vault_item::move_folder_contents_use_case::MoveFolderContentsUseCase;
use crate::application::use_cases::vault_item::move_notes_use_case::MoveNotesUseCase;
use crate::application::use_cases::vault_item::search_folders_use_case::SearchFoldersUseCase;
use crate::application::use_cases::version::get_vault_version_state_use_case::GetVaultVersionStateUseCase;
use crate::domain::note::note_content::NoteContent;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::pdf::pdf_document_repository::PdfDocumentRepository;
use crate::domain::search::full_text_index::FullTextIndex;
use crate::domain::vault::vault_item_repository::VaultItemRepository;
use crate::domain::version::version_control::VersionControl;
use crate::infrastructure::entrypoint::mcp::dto::*;
use serde_json::{Value, json};
use similar::{ChangeTag, TextDiff};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Genera un Unified Diff entre texto anterior y nuevo texto.
pub fn generate_unified_diff(old_text: &str, new_text: &str, file_name: &str) -> String {
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut output = String::new();
    let groups = diff.grouped_ops(3);
    if groups.is_empty() {
        return output;
    }
    output.push_str(&format!("--- a/{}\n+++ b/{}\n", file_name, file_name));
    for group in groups {
        for op in group {
            for change in diff.iter_changes(&op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                output.push_str(&format!("{}{}", sign, change.value()));
            }
        }
    }
    output
}

/// Lista de textos de un argumento JSON (`["a", "b"]`); vacía si falta.
fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Fecha legible de un instante unix: la hora local (con `date`, en Linux/macOS) o, si no se
/// puede, en UTC (`2026-11-18 14:05:09 UTC`).
fn readable_date(secs: u64) -> String {
    #[cfg(unix)]
    if let Ok(out) = std::process::Command::new("date")
        .args(["-d", &format!("@{secs}"), "+%Y-%m-%d %H:%M:%S %Z"])
        .output()
        && out.status.success()
    {
        let local = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !local.is_empty() {
            return local;
        }
    }
    utc_date(secs)
}

/// `AAAA-MM-DD hh:mm:ss UTC` sin librerías de fechas (algoritmo de días civiles de H. Hinnant).
fn utc_date(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Extensiones en minúsculas y sin punto (`.ArchiMate` → `archimate`).
fn extension_list(value: Option<&Value>) -> Vec<String> {
    string_list(value)
        .into_iter()
        .map(|e| e.trim().trim_start_matches('.').to_lowercase())
        .filter(|e| !e.is_empty())
        .collect()
}

/// Respuesta de texto con el JSON formateado; si falla la serialización, error con el prefijo.
fn to_text<T: serde::Serialize + ?Sized>(payload: &T, error_prefix: &str) -> McpCallToolResult {
    match serde_json::to_string_pretty(payload) {
        Ok(text) => McpCallToolResult::text(text),
        Err(e) => McpCallToolResult::error(format!("{}{}", error_prefix, e)),
    }
}

/// Bóveda por nombre (sin distinguir mayúsculas) o por ruta.
fn resolve_vault(vaults: &[VaultRef], wanted: &str) -> Result<VaultRef, String> {
    let w = wanted.trim();
    let by_path = std::path::Path::new(w);
    let found: Vec<&VaultRef> = vaults
        .iter()
        .filter(|v| v.name.eq_ignore_ascii_case(w) || v.path == by_path)
        .collect();
    match found.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(format!(
            "No vault named «{w}». Available: {}",
            vaults
                .iter()
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        many => Err(format!(
            "Multiple vaults match «{w}»; use its path: {}",
            many.iter()
                .map(|v| v.path.to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn vault_access_path(vault: &std::path::Path) -> PathBuf {
    vault.join(".synapse").join("vault-access.json")
}

/// Permisos de esta bóveda para consultar otras: ruta → permitido.
fn read_vault_access(vault: &std::path::Path) -> std::collections::BTreeMap<String, bool> {
    std::fs::read_to_string(vault_access_path(vault))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| serde_json::from_value(v["consult"].clone()).ok())
        .unwrap_or_default()
}

fn write_vault_access(vault: &std::path::Path, access: &std::collections::BTreeMap<String, bool>) {
    let path = vault_access_path(vault);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(&json!({ "consult": access })) {
        let _ = std::fs::write(path, text);
    }
}

/// Traduce las herramientas MCP (JSON-RPC) a casos de uso. No contiene reglas de negocio.
pub struct McpToolHandler<R: NoteRepository, I: FullTextIndex + ?Sized, G: VersionControl> {
    vault_path: PathBuf,
    note_repository: R,
    full_text_index: Option<Arc<I>>,
    git_service: Option<Arc<G>>,
    vault_item_repository: Arc<dyn VaultItemRepository>,
    pdf_document_repository: Arc<dyn PdfDocumentRepository>,
    /// Quién dibuja los diagramas como imagen (la ventana de Synapse); `None` sin ventana.
    diagram_renderer: Option<Arc<dyn DiagramRenderer>>,
    /// Conversaciones con agentes abiertas en la ventana, y cuál es la que llama.
    conversation_hub: Option<Arc<dyn ConversationHub>>,
    caller: Option<String>,
    /// Otras bóvedas del usuario que se pueden consultar (solo lectura) y a quién pedir permiso.
    vault_catalog: Option<Arc<dyn VaultCatalog>>,
    prompter: Option<Arc<dyn UserPrompter>>,
    /// Quien llama es un agente abierto por Synapse (trajo su token de sesión).
    private_tools: bool,
    /// Ayuda a configurar lo que el usuario tiene abierto en la ventana; `None` sin ventana.
    config: Option<Arc<ConfigAssistant>>,
}

/// Herramientas de solo lectura que aceptan `vault` para consultar otra bóveda.
pub const CROSS_VAULT_TOOLS: &[&str] = &[
    "search_notes",
    "full_text_search",
    "search_in_note",
    "read_note",
    "list_folder_contents",
    "search_folders",
    "list_sibling_notes",
    "get_backlinks_and_relations",
    "read_pdf_document",
    "extract_note_diagrams",
    "inspect_archimate_element",
    "git_file_history",
    "git_diff_revisions",
    "git_read_revision",
];

/// Líneas máximas de un diff o de un archivo histórico devueltas al agente de una vez.
const GIT_MAX_LINES: usize = 1500;

impl<R: NoteRepository, I: FullTextIndex + ?Sized, G: VersionControl> McpToolHandler<R, I, G> {
    pub fn new(
        vault_path: PathBuf,
        note_repository: R,
        full_text_index: Option<Arc<I>>,
        git_service: Option<Arc<G>>,
        vault_item_repository: Arc<dyn VaultItemRepository>,
        pdf_document_repository: Arc<dyn PdfDocumentRepository>,
    ) -> Self {
        Self {
            vault_path,
            note_repository,
            full_text_index,
            git_service,
            vault_item_repository,
            pdf_document_repository,
            diagram_renderer: None,
            conversation_hub: None,
            caller: None,
            vault_catalog: None,
            prompter: None,
            private_tools: false,
            config: None,
        }
    }

    pub fn with_config_assistant(mut self, assistant: Option<Arc<ConfigAssistant>>) -> Self {
        self.config = assistant;
        self
    }

    /// Ofrece las herramientas privadas: solo a quien trae el token de una sesión de Synapse.
    pub fn with_private_tools(mut self, allowed: bool) -> Self {
        self.private_tools = allowed;
        self
    }

    /// Permite consultar otras bóvedas (`list_vaults` y el parámetro `vault`); `prompter` pregunta
    /// al usuario la primera vez que esta bóveda consulta otra (sin él, solo se permiten las ya autorizadas).
    pub fn with_vaults(
        mut self,
        catalog: Option<Arc<dyn VaultCatalog>>,
        prompter: Option<Arc<dyn UserPrompter>>,
    ) -> Self {
        self.vault_catalog = catalog;
        self.prompter = prompter;
        self
    }

    /// Permite hablar con otras conversaciones de agentes (`list_conversations`,
    /// `send_to_conversation`); `caller` es la conversación que usa este MCP.
    pub fn with_conversations(
        mut self,
        hub: Option<Arc<dyn ConversationHub>>,
        caller: Option<String>,
    ) -> Self {
        self.conversation_hub = hub;
        self.caller = caller;
        self
    }

    /// Permite devolver diagramas como imagen (`render_diagram`).
    pub fn with_diagram_renderer(mut self, renderer: Option<Arc<dyn DiagramRenderer>>) -> Self {
        self.diagram_renderer = renderer;
        self
    }

    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Maneja la llamada de inicialización de MCP
    pub fn initialize(&self) -> McpInitializeResult {
        McpInitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: McpServerCapabilities {
                tools: Some(McpToolsCapability {
                    list_changed: false,
                }),
                resources: Some(McpResourcesCapability {
                    subscribe: false,
                    list_changed: false,
                }),
                prompts: Some(McpPromptsCapability {
                    list_changed: false,
                }),
            },
            server_info: McpServerInfo {
                name: "synapse-mcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    /// Retorna la lista de herramientas disponibles (Catálogo Unificado de 14 herramientas)
    pub fn list_tools(&self) -> McpToolsListResult {
        let mut result = McpToolsListResult {
            tools: vec![
                // 1. search_notes
                McpTool {
                    name: "search_notes".to_string(),
                    description: "Searches notes by name, path, or title within the vault, with optional tag filters (supporting hierarchical sub-tags) and snippets. Markdown only by default; pass 'extensions' to search other files (e.g. [\"archimate\"], [\"d2\", \"mmd\"]).".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "default": "",
                                "description": "Text or keyword to filter notes (can be left empty when using tag filters)"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional OKF tag or hierarchical sub-tag filter (e.g. 'cachyos' matches 'linux/cachyos')"
                            },
                            "limit": {
                                "type": "number",
                                "description": "Maximum number of results to return (default 20)"
                            },
                            "extensions": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "File extensions to scan without leading dot (e.g. [\"archimate\"] or [\"md\", \"d2\"]). Markdown only by default."
                            }
                        }
                    }),
                },
                // 2. full_text_search
                McpTool {
                    name: "full_text_search".to_string(),
                    description: "Deep full-text search (BM25) across the body of all indexed documents (Markdown, diagrams, code, PDF, Word, draw.io —including inside .drawio.png/.drawio.svg images—, ArchiMate .archimate models, and BPMN .bpmn processes), with folder, extension, tag filters, and token telemetry. Syntax: words use AND by default; multiple alternatives in one query with `|` or `OR` (`TIEL|QTI1|QTI3`, `(TIEL|QTI1) sender`); `\"exact phrase\"`; `-exclude`; `prefix*` (`BCTITP*`); fields `title:`, `hs:` (headings), `path:`, `kind:` (`kind:archimate`). With `lines: true` each hit includes matching lines with their location, like `rg -n`: `{line}` (1-based line number) in Markdown, code, and text diagrams; `{page}` in PDF (read with read_pdf_document start_page/end_page); `{section, paragraph}` in Word (section title and non-empty paragraph number); `{sheet, row, column}` in spreadsheets; `{diagram}` (page) in draw.io; `{element, type}` in ArchiMate and BPMN; `null` in Excalidraw. Long lines are trimmed around the match. Results with `truncated: true` were truncated when indexed (very large document): more matches may exist beyond the cut. With `exact: true`, the query is literal strings separated by `|` (`outbounds-ach|/ach-operations/`) and only documents containing them verbatim (case-insensitive) match, useful for endpoints, IDs, or codes with punctuation; matched against indexed text (in Markdown, stripped of formatting: link URLs do not count).".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Phrase or keywords to search inside document bodies"
                            },
                            "folder_path": {
                                "type": "string",
                                "description": "Subdirectory to restrict search scope"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Filter by tags present in notes"
                            },
                            "limit": {
                                "type": "number",
                                "description": "Maximum number of results (default 20)"
                            },
                            "extensions": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Only files with these extensions, without leading dot (e.g. [\"archimate\"])"
                            },
                            "lines": {
                                "type": "boolean",
                                "default": false,
                                "description": "Include matching lines with locations (`lines`, `total_line_matches`) in each hit, like `rg -n`; max 200 lines total (`lines_truncated` indicates trimming)"
                            },
                            "context_lines": {
                                "type": "number",
                                "default": 0,
                                "description": "With `lines`: lines before and after each match (max 5)"
                            },
                            "max_lines_per_doc": {
                                "type": "number",
                                "default": 20,
                                "description": "With `lines`: max lines per document (max 100)"
                            },
                            "exact": {
                                "type": "boolean",
                                "default": false,
                                "description": "Treat query as literal texts separated by `|`; only documents containing them verbatim (case-insensitive)"
                            }
                        },
                        "required": ["query"]
                    }),
                },
                // 3. search_in_note
                McpTool {
                    name: "search_in_note".to_string(),
                    description: "Searches one or more strings inside known notes (case-insensitive) and returns matching lines with line numbers, context, and occurrence counts. Multiple alternatives separated by `|`, like `rg \"a|b\"`: `id_ach_operation|DB limit|not stored`; each line indicates which ones matched in `matched`. If splitting on `|` produces an empty part (e.g. a Markdown table row `| field |`), the whole query is treated literally; `\\|` escapes a literal `|`, and `literal: true` never splits. Use it to verify recent edits (`counts`, `counts_only`) or inspect multiple notes at once (`relative_paths`); to search the whole vault, use full_text_search.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the note to search"
                            },
                            "relative_paths": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Multiple notes at once (max 20); returns `files` with one entry per note, and missing notes report their own `error` without failing the rest"
                            },
                            "query": {
                                "type": "string",
                                "description": "Text to search; multiple alternatives separated by `|` (e.g. `TIEL|QTI1|QTI3`)"
                            },
                            "literal": {
                                "type": "boolean",
                                "default": false,
                                "description": "Search 'query' literally even if it contains `|`"
                            },
                            "counts_only": {
                                "type": "boolean",
                                "default": false,
                                "description": "Only counts per search term (`counts`: matching lines and occurrences), without line snippets, like `grep -c`"
                            },
                            "context_lines": {
                                "type": "number",
                                "default": 2,
                                "description": "Number of lines before and after each match (max 10, default 2)"
                            },
                            "max_matches": {
                                "type": "number",
                                "description": "Max matches to return per note (default all); total_matches reports how many exist"
                            }
                        },
                        "required": ["query"]
                    }),
                },
                // 4. list_folder_contents
                McpTool {
                    name: "list_folder_contents".to_string(),
                    description: "Lists notes and/or subdirectories inside a vault folder, including line counts and estimated tokens.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "folder_path": {
                                "type": "string",
                                "description": "Relative folder path to inspect (e.g. 'projects/rust'). Leave empty for vault root."
                            },
                            "include_notes": {
                                "type": "boolean",
                                "description": "Include notes and files inside the folder (default true)"
                            },
                            "include_folders": {
                                "type": "boolean",
                                "description": "Include subdirectories inside the folder (default true)"
                            },
                            "recursive": {
                                "type": "boolean",
                                "description": "If true, lists subfolders recursively (default false)"
                            },
                            "limit": {
                                "type": "number",
                                "default": 50,
                                "description": "Maximum number of notes returned (default 50, max 200)"
                            },
                            "offset": {
                                "type": "number",
                                "default": 0,
                                "description": "Pagination offset for notes (default 0)"
                            }
                        }
                    }),
                },
                // 4.1. search_folders
                McpTool {
                    name: "search_folders".to_string(),
                    description: "Searches folders by fuzzy name or path match, returning a semantic fingerprint (note count, dominant tags, and sample notes) to disambiguate similarly named folders.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search term or partial folder name (e.g. 'azure', 'invoices', 'cli')"
                            },
                            "limit": {
                                "type": "number",
                                "default": 20,
                                "description": "Max matching folders to return (default 20)"
                            },
                            "offset": {
                                "type": "number",
                                "default": 0,
                                "description": "Pagination offset (default 0)"
                            }
                        }
                    }),
                },
                // 4.2. move_folder_contents
                McpTool {
                    name: "move_folder_contents".to_string(),
                    description: "Moves all notes from a source folder to a target folder, with conflict strategy, extension filter, and dry_run preview.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "source_folder": {
                                "type": "string",
                                "description": "Relative source folder path (e.g. 'inbox' or 'projects/azure')"
                            },
                            "target_folder": {
                                "type": "string",
                                "description": "Relative target folder path (e.g. 'archive/azure' or 'clients/bcp')"
                            },
                            "extension": {
                                "type": "string",
                                "description": "Optional file extension filter to move (e.g. 'md' or 'pdf')"
                            },
                            "conflict_strategy": {
                                "type": "string",
                                "enum": ["skip", "rename", "overwrite"],
                                "default": "skip",
                                "description": "Conflict handling when destination exists: 'skip', 'rename' (append numeric suffix), or 'overwrite'"
                            },
                            "dry_run": {
                                "type": "boolean",
                                "default": false,
                                "description": "If true, simulates and returns files that would move without modifying disk"
                            }
                        },
                        "required": ["source_folder", "target_folder"]
                    }),
                },
                // 5. list_sibling_notes
                McpTool {
                    name: "list_sibling_notes".to_string(),
                    description: "Given a reference note, lists all other notes located in the same folder.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the reference note (e.g. 'notes/architecture.md')"
                            },
                            "include_self": {
                                "type": "boolean",
                                "description": "Also include the reference note in the result (default false)"
                            },
                            "include_folders": {
                                "type": "boolean",
                                "description": "Include sibling subfolders in the same directory (default false)"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 6. read_note
                McpTool {
                    name: "read_note".to_string(),
                    description: "Reads a Markdown note and its OKF metadata. Supports 'full', 'chunk', 'section', and 'metadata_only'/'outline_only' modes, providing content_hash (OCC), TOC, and block counts. Also appends the root and folder OKF index.md (vault maps maintained by Synapse; each folder has its own on disk).".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative file path in the vault (e.g. 'notes/architecture.md')"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["full", "chunk", "section", "metadata_only", "outline_only"],
                                "default": "full",
                                "description": "'full': reads entire note. 'chunk': reads a line range. 'section': extracts one heading section. 'metadata_only'/'outline_only': OKF metadata, TOC, and stats without the body."
                            },
                            "outline_only": {
                                "type": "boolean",
                                "description": "If true, returns only TOC (headings with line numbers) and metadata without the body"
                            },
                            "section": {
                                "type": "string",
                                "description": "Heading name to extract (direct alias of section_heading)"
                            },
                            "start_line": {
                                "type": "number",
                                "description": "Start line (1-based) when mode='chunk' (default 1)"
                            },
                            "line_count": {
                                "type": "number",
                                "description": "Number of lines to read in mode='chunk' (default 150, max 500)"
                            },
                            "section_heading": {
                                "type": "string",
                                "description": "Heading text to extract when mode='section' (e.g. '## ACP Channel')"
                            },
                            "include_line_numbers": {
                                "type": "boolean",
                                "default": false,
                                "description": "Prefix each returned line with its line number (e.g. '42: text') to assist patch_note."
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 7. write_note
                McpTool {
                    name: "write_note".to_string(),
                    description: "Creates a note or replaces an existing one, with OCC hash protection, anti-truncation guard, and dry_run mode. Overwriting requires expected_hash (recommended) or overwrite: true; otherwise it fails. Response indicates in 'created' whether the note is new and in 'warning' if its parent folder did not exist.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, true)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative file path (e.g. 'projects/new.md')"
                            },
                            "title": {
                                "type": "string",
                                "description": "Document title (optional if the note already exists)"
                            },
                            "content": {
                                "type": "string",
                                "description": "Markdown body"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Taxonomy tags"
                            },
                            "aliases": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Alternative names for the note"
                            },
                            "expected_hash": {
                                "type": "string",
                                "description": "Hash previously obtained from read_note for optimistic concurrency control"
                            },
                            "overwrite": {
                                "type": "boolean",
                                "default": false,
                                "description": "Allows replacing an existing note without reading it first. Prefer reading it and sending expected_hash, which suffices on its own. Without either, writing over an existing note fails"
                            },
                            "allow_large_overwrite": {
                                "type": "boolean",
                                "default": false,
                                "description": "Allows overwriting even if it shrinks the original document (>150 lines) by more than 40%"
                            },
                            "dry_run": {
                                "type": "boolean",
                                "default": false,
                                "description": "If true, validates and returns unified_diff without modifying disk"
                            }
                        },
                        "required": ["relative_path", "content"]
                    }),
                },
                // 8. patch_note
                McpTool {
                    name: "patch_note".to_string(),
                    description: "Surgically modifies a note snippet by replacing an exact text block with fuzzy anchoring tolerance and OCC check.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the note to edit"
                            },
                            "old_text": {
                                "type": "string",
                                "description": "Exact original text to replace"
                            },
                            "new_text": {
                                "type": "string",
                                "description": "New Markdown text replacing old_text"
                            },
                            "start_line": {
                                "type": "number",
                                "description": "Estimated start line of the block (1-based)"
                            },
                            "end_line": {
                                "type": "number",
                                "description": "Estimated end line of the block"
                            },
                            "expected_hash": {
                                "type": "string",
                                "description": "Previous hash to verify the note was not modified concurrently"
                            },
                            "dry_run": {
                                "type": "boolean",
                                "default": false,
                                "description": "If true, computes unified_diff without modifying disk"
                            }
                        },
                        "required": ["relative_path", "old_text", "new_text"]
                    }),
                },
                // 9. append_to_note
                McpTool {
                    name: "append_to_note".to_string(),
                    description: "Appends content to the end of a note or inside a specific Markdown heading section.".to_string(),
                    annotations: Some(McpToolAnnotations::write(false, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the note"
                            },
                            "content": {
                                "type": "string",
                                "description": "Markdown text to insert"
                            },
                            "section_heading": {
                                "type": "string",
                                "description": "Optional heading to insert into (e.g. '## Tasks'). If omitted, appends at end of file."
                            },
                            "position": {
                                "type": "string",
                                "enum": ["start_of_section", "end_of_section"],
                                "default": "end_of_section",
                                "description": "Position within the target section"
                            }
                        },
                        "required": ["relative_path", "content"]
                    }),
                },
                // 10. update_note_metadata
                McpTool {
                    name: "update_note_metadata".to_string(),
                    description: "Atomically updates OKF metadata (title, tags, aliases, custom_properties) in YAML frontmatter without modifying the note body.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, true)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the note"
                            },
                            "title": {
                                "type": "string",
                                "description": "Optional new title"
                            },
                            "add_tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Tags to add (deduplicated)"
                            },
                            "remove_tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Tags to remove"
                            },
                            "aliases": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "List of aliases to set"
                            },
                            "custom_properties": {
                                "type": "object",
                                "description": "Additional key-value pairs for frontmatter"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 11. move_notes
                McpTool {
                    name: "move_notes".to_string(),
                    description: "Moves notes to a target folder with optional automatic wikilink rewriting across the vault.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "notes": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "List of note paths to move"
                            },
                            "target_folder": {
                                "type": "string",
                                "description": "Relative destination folder"
                            },
                            "create_target_folder": {
                                "type": "boolean",
                                "default": true,
                                "description": "Create destination folder if missing"
                            },
                            "overwrite": {
                                "type": "boolean",
                                "default": false,
                                "description": "Overwrite if file already exists at destination"
                            },
                            "update_wikilinks": {
                                "type": "boolean",
                                "default": false,
                                "description": "Rewrite incoming links in other notes to point to the new location"
                            }
                        },
                        "required": ["notes", "target_folder"]
                    }),
                },
                // 12. delete_note
                McpTool {
                    name: "delete_note".to_string(),
                    description: "Permanently deletes a note from the vault filesystem. Supports optimistic concurrency control via expected_hash.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the note to delete inside the vault (e.g. 'folder/note.md')"
                            },
                            "expected_hash": {
                                "type": "string",
                                "description": "Optional SHA-256 hash (from read_note). Prevents deletion if the content changed"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 13. get_backlinks_and_relations
                McpTool {
                    name: "get_backlinks_and_relations".to_string(),
                    description: "Returns backlinks with contextual snippets, outlinks (resolved, unresolved, and ambiguous: when multiple notes share a name, `ambiguous` lists candidates and linking by full path is recommended), and top 5 neighbors by Jaccard tag similarity.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the note to analyze"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 14. get_vault_git_status
                McpTool {
                    name: "get_vault_git_status".to_string(),
                    description: "Returns Git version control status for the vault (branch, index, worktree, stash). When repositories live in subfolders, `repos` lists each one (folder, branch, changes) while `statuses` paths remain vault-relative.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                // 15. read_pdf_document
                McpTool {
                    name: "read_pdf_document".to_string(),
                    description: "Inspects PDF documents: outline/metadata, page-range reading (max 5 pages), or internal text search.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the PDF file (e.g. 'docs/paper.pdf')"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["outline_only", "pages", "search"],
                                "default": "outline_only",
                                "description": "'outline_only': page count and metadata. 'pages': extracts text between start_page and end_page. 'search': searches for a query inside the PDF."
                            },
                            "start_page": {
                                "type": "number",
                                "description": "Start page (1-based, default 1)"
                            },
                            "end_page": {
                                "type": "number",
                                "description": "End page (max 5 pages per call)"
                            },
                            "search_query": {
                                "type": "string",
                                "description": "Search term when mode='search'"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 16. extract_note_diagrams
                McpTool {
                    name: "extract_note_diagrams".to_string(),
                    description: "Extracts all diagram code blocks (Mermaid, D2) from a note with line numbers, or reads .d2 / .mmd files. For a draw.io diagram (.drawio, or embedded inside .drawio.png/.drawio.svg), returns each page's containers with their shapes, standalone shapes, and edges (from → to, with labels); pass `revision` to inspect it at that commit.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the Markdown note, .d2 / .mmd file, or draw.io diagram (.drawio, .drawio.png, .drawio.svg)"
                            },
                            "revision": {
                                "type": "string",
                                "description": "draw.io only: Git commit (hash, HEAD~1…) to inspect the diagram as of that revision"
                            },
                            "diagram_type": {
                                "type": "string",
                                "enum": ["all", "mermaid", "d2"],
                                "default": "all",
                                "description": "Filter by diagram type"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // Modelos ArchiMate (`.archimate` de Archi)
                McpTool {
                    name: "inspect_archimate_element".to_string(),
                    description: "Searches elements in an ArchiMate model (.archimate) by exact ID or partial name and returns in one response their type, line, documentation, properties (e.g. UNIQUECODE), incoming and outgoing relations with the other endpoint's name and type, and the views (ArchimateDiagramModel) where they appear. For folder and view outlines, use read_note with mode \"outline_only\".".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the .archimate model"
                            },
                            "query": {
                                "type": "string",
                                "description": "Element ID (id-…) or part of its name, case-insensitive"
                            },
                            "limit": {
                                "type": "number",
                                "description": "Max elements to return (default 5)"
                            }
                        },
                        "required": ["relative_path", "query"]
                    }),
                },
                // 18. report_feedback
                McpTool {
                    name: "report_feedback".to_string(),
                    description: "Leaves a note for Synapse developers when this MCP lacks something needed to complete a task (a tool, parameter, or format). Appends to feedback.md in global config; does not touch the vault.".to_string(),
                    annotations: Some(McpToolAnnotations::write(false, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "Short summary of what is missing"
                            },
                            "details": {
                                "type": "string",
                                "description": "What you were trying to do, what was missing, and how you worked around it"
                            }
                        },
                        "required": ["title", "details"]
                    }),
                },
                // 24. git_file_history
                McpTool {
                    name: "git_file_history".to_string(),
                    description: "Git history of a vault file (following renames; uses the nearest repository containing it, including in subfolders): hash, short hash, date, author, message, and tags for each commit, newest first. Use it to pick commits to compare with git_diff_revisions.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {"type": "string", "description": "Relative file path"},
                            "limit": {"type": "integer", "description": "Max commits (default 20)", "default": 20}
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 25. git_diff_revisions
                McpTool {
                    name: "git_diff_revisions".to_string(),
                    description: "Shows what changed in a file between two commits: unified diff (from `from` to `to`; if `to` omitted, against the working tree), with lines added and removed. If renamed across commits, resolves the name in each commit. For binary images and diagrams, use render_diagram with `revision` on each version.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {"type": "string", "description": "Current relative path of the file"},
                            "from": {"type": "string", "description": "Start commit (hash, short hash, HEAD~2…)"},
                            "to": {"type": "string", "description": "End commit; empty = local file on disk"}
                        },
                        "required": ["relative_path", "from"]
                    }),
                },
                // 26. git_read_revision
                McpTool {
                    name: "git_read_revision".to_string(),
                    description: "Reads a file's contents at a commit (using its path at that time if renamed). Returns a line range; use start_line and line_count for long files.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {"type": "string", "description": "Current relative path of the file"},
                            "revision": {"type": "string", "description": "Commit (hash, short hash, HEAD~1…)"},
                            "start_line": {"type": "integer", "description": "First line (1-based)", "default": 1},
                            "line_count": {"type": "integer", "description": "Lines to return (default 300)", "default": 300}
                        },
                        "required": ["relative_path", "revision"]
                    }),
                },
                // 23. read_replies
                McpTool {
                    name: "read_replies".to_string(),
                    description: "For external MCP clients (not a Synapse panel conversation, e.g. Claude Code): collects replies from conversations queried with send_to_conversation (expects_reply=true). Each reply is delivered once; if none yet, call again later.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({"type": "object", "properties": {}}),
                },
                // 22. list_vaults
                McpTool {
                    name: "list_vaults".to_string(),
                    description: "Lists the user's other Synapse vaults (open and recent) that can be queried read-only by passing `vault` (vault name) to search_notes, full_text_search, read_note, list_folder_contents, and other read tools. Use when the information you need lives in another vault. The first query to a vault asks the user for permission.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({"type": "object", "properties": {}}),
                },
                // 20. list_conversations
                McpTool {
                    name: "list_conversations".to_string(),
                    description: "Lists open agent conversations in the Synapse panel (including other agents: Claude, Gemini…), both in this window (#1, #2…) and in other vaults open in other windows (mainframe#1…, or mainframe#new if none exists). Yours is marked `is_you`. Call before send_to_conversation to pick the target.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({"type": "object", "properties": {}}),
                },
                // 21. send_to_conversation
                McpTool {
                    name: "send_to_conversation".to_string(),
                    description: "Sends a message to another agent conversation in the panel (e.g. when the user says \"ask Gemini…\" or \"tell #2…\"). Asynchronous: if the target is busy, the message is delivered when its turn ends. With expects_reply=true, the reply will arrive later as a new message in this conversation; do not wait for or fabricate it. If unsure which conversation the user means, ask.".to_string(),
                    annotations: Some(McpToolAnnotations::write(false, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "to": {"type": "string", "description": "Target: #2 (this window), part of its title or agent name; or another vault: \"mainframe\" (its conversation for you, or a fresh one), \"mainframe#1\" (that conversation), or \"mainframe#new\""},
                            "message": {"type": "string", "description": "What to say or ask, including any needed context (it cannot see your conversation)"},
                            "expects_reply": {"type": "boolean", "description": "true if this is a question and you want its reply back", "default": false}
                        },
                        "required": ["to", "message"]
                    }),
                },
                // 19. render_diagram
                McpTool {
                    name: "render_diagram".to_string(),
                    description: "Returns a diagram or image from the vault as an image, as rendered in Synapse: Excalidraw, Mermaid, D2, and PlantUML (rendered as PNG), draw.io (.drawio.png, .drawio.svg), and images (.png, .jpg, .svg, .webp…). Use it to view an attached image or diagram, or when read_note indicates `diagram.render_tool`. For text and structure of a draw.io diagram, use extract_note_diagrams. Requires the Synapse window to be open.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Relative path of the diagram in the vault"
                            },
                            "vault": {
                                "type": "string",
                                "description": "Another vault (see list_vaults); empty = this vault"
                            },
                            "revision": {
                                "type": "string",
                                "description": "Git commit (hash, HEAD~1…): the diagram or image (e.g. .drawio.png) as it was at that commit"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 17. list_vault_tree (alias retrocompatible)
                McpTool {
                    name: "list_vault_tree".to_string(),
                    description: "Backward-compatible alias for list_folder_contents.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "parent_path": {
                                "type": "string",
                                "description": "Relative folder"
                            }
                        }
                    }),
                },
            ],
        };
        // Las herramientas de lectura aceptan `vault` para consultar otra bóveda
        for tool in result
            .tools
            .iter_mut()
            .filter(|t| CROSS_VAULT_TOOLS.contains(&t.name.as_str()))
        {
            if let Some(props) = tool
                .input_schema
                .get_mut("properties")
                .and_then(|p| p.as_object_mut())
            {
                props.insert(
                    "vault".into(),
                    json!({"type": "string", "description": "Another vault to query (its name, see list_vaults); empty = this vault"}),
                );
            }
        }
        // `report_feedback` solo existe en las compilaciones con la feature `develop`
        if !cfg!(feature = "develop") {
            result.tools.retain(|t| t.name != "report_feedback");
        }
        if self.private_tools {
            result.tools.extend(config_tools::definitions());
        }
        result
    }

    /// Ejecuta una herramienta MCP por nombre
    pub fn call_tool(&self, name: &str, arguments: Option<Value>) -> McpCallToolResult {
        let mut args = arguments.unwrap_or(Value::Object(serde_json::Map::new()));
        // `vault`: la misma herramienta, en solo lectura, sobre otra bóveda
        let target = args
            .as_object_mut()
            .and_then(|o| o.remove("vault"))
            .and_then(|v| v.as_str().map(str::to_string))
            .filter(|v| !v.trim().is_empty());
        if let Some(vault) = target {
            return self.call_in_other_vault(name, &vault, args);
        }
        let result = self.dispatch(name, &args);
        // `write_note` crea rutas nuevas: que no existan no es un error de ubicación
        if name == "write_note" {
            return result;
        }
        self.with_location_hint(&args, result)
    }

    fn dispatch(&self, name: &str, args: &Value) -> McpCallToolResult {
        match name {
            "search_notes" => self.execute_search_notes(args),
            "search_folders" => self.execute_search_folders(args),
            "move_folder_contents" => self.execute_move_folder_contents_tool(args),
            "full_text_search" => self.execute_full_text_search(args),
            "search_in_note" => self.execute_search_in_note(args),
            "list_folder_contents" => self.execute_list_folder_contents_tool(args),
            "list_vault_tree" => {
                let parent = args
                    .get("parent_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.execute_list_folder_contents(parent, true, true, false, None, None, None)
            }
            "list_sibling_notes" => self.execute_list_sibling_notes(args),
            "read_note" => self.with_okf_indexes(args, self.execute_read_note(args)),
            "write_note" => self.execute_write_note(args),
            "patch_note" => self.execute_patch_note(args),
            "append_to_note" => self.execute_append_to_note(args),
            "update_note_metadata" => self.execute_update_note_metadata(args),
            "move_notes" => self.execute_move_notes_tool(args),
            "delete_note" => self.execute_delete_note(args),
            "get_backlinks_and_relations" => self.execute_get_backlinks_and_relations(args),
            "get_vault_git_status" => self.execute_get_vault_git_status(),
            "read_pdf_document" => self.execute_read_pdf_document(args),
            "extract_note_diagrams" => self.execute_extract_note_diagrams(args),
            "inspect_archimate_element" => self.execute_inspect_archimate(args),
            "render_diagram" => self.execute_render_diagram(args),
            "list_conversations" => self.execute_list_conversations(),
            "list_vaults" => self.execute_list_vaults(),
            "git_file_history" => self.execute_git_file_history(args),
            "git_diff_revisions" => self.execute_git_diff_revisions(args),
            "git_read_revision" => self.execute_git_read_revision(args),
            "read_replies" => match self.conversations() {
                Ok(hub) => match hub.replies() {
                    Ok(v) => to_text(&v, ""),
                    Err(e) => McpCallToolResult::error(e),
                },
                Err(e) => e,
            },
            "send_to_conversation" => self.execute_send_to_conversation(args),
            "report_feedback" if cfg!(feature = "develop") => self.execute_report_feedback(args),
            other if other.starts_with(PRIVATE_TOOL_PREFIX) => {
                self.execute_config_tool(other, args)
            }
            other => McpCallToolResult::error(format!("Unknown tool: '{}'", other)),
        }
    }

    fn execute_config_tool(&self, name: &str, args: &Value) -> McpCallToolResult {
        if !self.private_tools {
            return McpCallToolResult::error(format!(
                "'{name}' is only available to agents launched by Synapse"
            ));
        }
        if !config_tools::definitions().iter().any(|tool| tool.name == name) {
            return McpCallToolResult::error(format!("Unknown tool: '{name}'"));
        }
        let Some(assistant) = &self.config else {
            return McpCallToolResult::error(NOTHING_OPEN);
        };
        match assistant.call(name, args) {
            Ok(value) => McpCallToolResult::text(format!("{value:#}")),
            Err(error) => McpCallToolResult::error(error),
        }
    }

    // ==========================================
    // Herramientas: traducen JSON ↔ Command/Query y delegan en casos de uso
    // ==========================================

    fn vault(&self) -> PathBuf {
        self.vault_path.clone()
    }

    /// Añade al resultado de `read_note` el `index.md` OKF de la raíz y el de la carpeta de la nota
    /// (cada uno hasta ~4 KB; si es más largo se corta y se indica la ruta para leerlo entero)
    fn with_okf_indexes(&self, args: &Value, mut result: McpCallToolResult) -> McpCallToolResult {
        const MAX: usize = 4096;
        let Some(rel) = args.get("relative_path").and_then(|v| v.as_str()) else {
            return result;
        };
        let rel = rel.trim_start_matches('/');
        if result.is_error || rel.rsplit('/').next() == Some(crate::domain::okf::INDEX_FILE) {
            return result;
        }
        let (root, own) = crate::infrastructure::okf::OkfIndexer::indexes_for(&self.vault(), rel);
        let cap = |text: String, path: &str| -> String {
            if text.len() <= MAX {
                return text;
            }
            let mut end = MAX;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}\n… (truncated; read it in full with read_note «{}»)", &text[..end], path)
        };
        let folder = rel.rfind('/').map(|i| &rel[..i]).unwrap_or("");
        let mut parts = Vec::new();
        if let Some(r) = root {
            parts.push(format!("## index.md (vault root)\n\n{}", cap(r, "index.md")));
        }
        if let Some(o) = own {
            let path = format!("{folder}/index.md");
            parts.push(format!("## {path} (note folder)\n\n{}", cap(o, &path)));
        }
        if !parts.is_empty() {
            result.content.push(crate::infrastructure::entrypoint::mcp::dto::McpContentItem {
                content_type: "text".into(),
                text: Some(format!(
                    "OKF indexes (index.md, generated by Synapse) to help orient within the vault:\n\n{}",
                    parts.join("\n\n")
                )),
                data: None,
                mime_type: None,
            });
        }
        result
    }

    fn execute_search_notes(&self, args: &Value) -> McpCallToolResult {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .map(|q| q.trim().to_lowercase())
            .unwrap_or_default();
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let tags = string_list(args.get("tags"));

        let query_text = query.clone();
        let no_extensions = extension_list(args.get("extensions")).is_empty();
        match SearchNotesUseCase::new(&self.note_repository).execute(SearchNotesQuery {
            vault_path: self.vault(),
            query,
            limit,
            tags,
            extensions: extension_list(args.get("extensions")),
        }) {
            Ok(hits) => {
                let filtered: Vec<_> = hits
                    .into_iter()
                    .map(|h| {
                        json!({
                            "title": h.title,
                            "relative_path": h.relative_path,
                            "tags": h.tags,
                            "aliases": h.aliases,
                            "description": h.description,
                            "snippet": h.snippet,
                            "total_lines": h.total_lines,
                            "estimated_tokens": h.estimated_tokens,
                        })
                    })
                    .collect();
                let mut result = to_text(&filtered, "Failed to serialize notes: ");
                if filtered.is_empty()
                    && !query_text.is_empty()
                    && no_extensions
                    && let Some(hint) = self.other_types_hint(&query_text)
                {
                    result.content.push(McpContentItem::text(hint));
                }
                result
            }
            Err(err) => McpCallToolResult::error(err),
        }
    }

    /// Si una herramienta falla por una ruta que no existe en esta bóveda, añade al error cuál es
    /// la bóveda activa, en qué otras bóvedas sí existe esa ruta y archivos con el mismo nombre
    /// aquí: los agentes suelen equivocarse de bóveda o de carpeta y acaban en `ls`/`fd`.
    fn with_location_hint(&self, args: &Value, mut result: McpCallToolResult) -> McpCallToolResult {
        if !result.is_error {
            return result;
        }
        let hint = ["relative_path", "folder_path", "parent_path"]
            .iter()
            .find_map(|k| args.get(*k).and_then(|v| v.as_str()))
            .and_then(|rel| self.location_hint(rel));
        if let Some(hint) = hint {
            result.content.push(McpContentItem::text(hint));
        }
        result
    }

    /// Pista para una ruta que no existe en esta bóveda (ver `with_location_hint`); `None` si
    /// la ruta existe o está vacía.
    fn location_hint(&self, rel: &str) -> Option<String> {
        let rel = rel.trim().trim_matches('/');
        if rel.is_empty() {
            return None;
        }
        let here = self.vault();
        if self.vault_item_repository.exists(&here, rel) {
            return None;
        }
        let here_canon = here.canonicalize().unwrap_or_else(|_| here.clone());
        let vaults = self.vault_catalog.as_ref().map(|c| c.list()).unwrap_or_default();
        let here_name = vaults
            .iter()
            .find(|v| v.path == here_canon || v.path == here)
            .map(|v| v.name.clone())
            .or_else(|| here.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default();
        let mut hint = format!(
            "Path «{rel}» does not exist in active vault «{here_name}» ({}). Paths are relative to its root.",
            here.display()
        );
        let elsewhere: Vec<&str> = vaults
            .iter()
            .filter(|v| v.path != here_canon && v.path != here && v.path.join(rel).exists())
            .map(|v| v.name.as_str())
            .collect();
        if !elsewhere.is_empty() {
            hint.push_str(&format!(
                " It exists in: {}. Retry the call with `vault` (e.g. vault: \"{}\").",
                elsewhere.iter().map(|n| format!("«{n}»")).collect::<Vec<_>>().join(", "),
                elsewhere[0]
            ));
        }
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let similar: Vec<String> = self
            .vault_item_repository
            .list_vault_files(&here, "")
            .unwrap_or_default()
            .into_iter()
            .filter(|f| {
                let p = f.relative_path.to_lowercase();
                p.rsplit('/').next() == Some(name.to_lowercase().as_str())
                    || p.ends_with(&format!("/{}", rel.to_lowercase()))
            })
            .take(5)
            .map(|f| f.relative_path)
            .collect();
        if !similar.is_empty() {
            hint.push_str(&format!(" This vault contains: {}.", similar.join(", ")));
        } else if elsewhere.is_empty() {
            hint.push_str(" Search for it with search_notes (using `extensions` if not Markdown) or list_folder_contents.");
        }
        Some(hint)
    }

    /// Si `search_notes` no encontró Markdown, cuántos archivos de otros tipos sí cumplen la
    /// búsqueda por ruta: sin esto, los agentes concluían que el archivo no existía.
    fn other_types_hint(&self, query: &str) -> Option<String> {
        let files = self.vault_item_repository.list_vault_files(&self.vault(), "").ok()?;
        let others: Vec<String> = files
            .into_iter()
            .map(|f| f.relative_path)
            .filter(|p| p.to_lowercase().contains(query))
            .filter(|p| {
                let lower = p.to_lowercase();
                !lower.ends_with(".md") && !lower.ends_with(".markdown")
            })
            .collect();
        if others.is_empty() {
            return None;
        }
        let mut exts: Vec<String> = others
            .iter()
            .filter_map(|p| p.rsplit('/').next()?.rsplit_once('.').map(|(_, e)| e.to_lowercase()))
            .collect();
        exts.sort();
        exts.dedup();
        Some(format!(
            "No Markdown notes matched, but there are {} file(s) of other types whose path contains «{query}» (e.g. {}). Retry with extensions: {:?}.",
            others.len(),
            others.iter().take(5).cloned().collect::<Vec<_>>().join(", "),
            exts
        ))
    }

    fn execute_search_folders(&self, args: &Value) -> McpCallToolResult {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .map(|q| q.trim().to_lowercase())
            .unwrap_or_default();
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let result =
            match SearchFoldersUseCase::new(&self.note_repository, &*self.vault_item_repository)
                .execute(SearchFoldersQuery {
                    vault_path: self.vault(),
                    query,
                    limit,
                    offset,
                }) {
                Ok(r) => r,
                Err(e) => return McpCallToolResult::error(e),
            };
        let has_more = offset + limit < result.total;
        let next_offset = if has_more { Some(offset + limit) } else { None };
        let payload = json!({
            "total_folders": result.total,
            "returned_folders": result.folders.len(),
            "offset": offset,
            "limit": limit,
            "has_more": has_more,
            "next_offset": next_offset,
            "folders": result.folders,
        });
        to_text(&payload, "Failed to serialize folders: ")
    }

    fn execute_move_folder_contents_tool(&self, args: &Value) -> McpCallToolResult {
        let source_folder = match args.get("source_folder").and_then(|v| v.as_str()) {
            Some(sf) => sf.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("Argument 'source_folder' is required"),
        };
        let target_folder = match args.get("target_folder").and_then(|v| v.as_str()) {
            Some(tf) => tf.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("Argument 'target_folder' is required"),
        };
        let command = MoveFolderContentsCommand {
            vault_path: self.vault(),
            source_folder: source_folder.clone(),
            target_folder: target_folder.clone(),
            extension: args
                .get("extension")
                .and_then(|v| v.as_str())
                .map(|e| e.trim_start_matches('.').to_lowercase()),
            conflict_strategy: args
                .get("conflict_strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("skip")
                .to_string(),
            dry_run: args
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };

        let payload = match MoveFolderContentsUseCase::new(&*self.vault_item_repository)
            .execute(command)
        {
            Err(e) => return McpCallToolResult::error(e),
            Ok(MoveFolderContentsResult::Preview { files }) => json!({
                "dry_run": true,
                "source_folder": source_folder,
                "target_folder": target_folder,
                "total_files_to_move": files.len(),
                "files": files,
            }),
            Ok(MoveFolderContentsResult::Done {
                moved,
                skipped,
                errors,
            }) => {
                let moved: Vec<_> = moved.into_iter().map(|(original, new_path)| json!({ "original": original, "new_path": new_path })).collect();
                json!({
                    "source_folder": source_folder,
                    "target_folder": target_folder,
                    "moved_count": moved.len(),
                    "skipped_count": skipped.len(),
                    "errors_count": errors.len(),
                    "moved_files": moved,
                    "skipped_files": skipped,
                    "errors": errors,
                })
            }
        };
        to_text(&payload, "")
    }

    fn execute_full_text_search(&self, args: &Value) -> McpCallToolResult {
        let query_str = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q.trim(),
            None => return McpCallToolResult::error("Argument 'query' is required"),
        };
        if query_str.is_empty() {
            return McpCallToolResult::text("[]");
        }
        let Some(ref fts) = self.full_text_index else {
            return McpCallToolResult::error(
                "The Tantivy full-text search index is not available.",
            );
        };
        let query = SearchNotesFullTextQuery {
            vault_path: self.vault(),
            text: query_str.to_string(),
            limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize,
            folder: args
                .get("folder_path")
                .and_then(|v| v.as_str())
                .map(|f| f.trim_matches('/').to_lowercase()),
            tags: string_list(args.get("tags")),
            extensions: extension_list(args.get("extensions")),
            lines: args.get("lines").and_then(|v| v.as_bool()).unwrap_or(false).then(|| {
                crate::domain::search::full_text::LineOptions {
                    context: args.get("context_lines").and_then(|v| v.as_u64()).unwrap_or(0).min(5) as usize,
                    max_per_doc: args
                        .get("max_lines_per_doc")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(20)
                        .clamp(1, 100) as usize,
                }
            }),
            exact: if args.get("exact").and_then(|v| v.as_bool()).unwrap_or(false) {
                crate::domain::search::alternatives::split_alternatives(query_str)
                    .into_iter()
                    .filter(|t| !t.trim().is_empty())
                    .collect()
            } else {
                vec![]
            },
        };
        let query_lines = query.lines.is_some();
        match SearchNotesFullTextUseCase::new(&**fts, &self.note_repository).execute(query) {
            Ok(result) => {
                /// Tope de líneas devueltas entre todos los resultados.
                const MAX_LINES_TOTAL: usize = 200;
                let mut lines_budget = MAX_LINES_TOTAL;
                let mut lines_truncated = false;
                let hits: Vec<_> = result
                    .hits
                    .into_iter()
                    .map(|h| {
                        let mut hit = json!({
                            "path": h.hit.note_path,
                            "title": h.hit.title,
                            "score": h.hit.score,
                            "heading": h.hit.heading,
                            "snippet": h.hit.snippet,
                            "total_lines": h.total_lines,
                            "estimated_tokens": h.estimated_tokens,
                            "matched_terms": h.hit.matched_terms,
                        });
                        if h.hit.truncated {
                            hit["truncated"] = json!(true);
                        }
                        if query_lines {
                            let mut lines = h.hit.lines;
                            if lines.len() > lines_budget {
                                lines.truncate(lines_budget);
                                lines_truncated = true;
                            }
                            lines_budget -= lines.len();
                            lines_truncated |= lines.len() < h.hit.total_line_matches;
                            hit["total_line_matches"] = json!(h.hit.total_line_matches);
                            hit["lines"] = json!(lines);
                        }
                        hit
                    })
                    .collect();
                let mut results = json!({
                    "total_matches": result.total,
                    "returned_matches": hits.len(),
                    "hits": hits,
                });
                if query_lines {
                    results["lines_truncated"] = json!(lines_truncated);
                }
                to_text(&results, "Failed to serialize: ")
            }
            Err(err) => McpCallToolResult::error(err),
        }
    }

    fn execute_search_in_note(&self, args: &Value) -> McpCallToolResult {
        const MAX_NOTES: usize = 20;
        let single = args.get("relative_path").and_then(|v| v.as_str());
        let many = string_list(args.get("relative_paths"));
        let relative_paths: Vec<String> = match (single, many.is_empty()) {
            (Some(p), true) => vec![p.to_string()],
            (None, false) => many,
            (Some(p), false) => std::iter::once(p.to_string()).chain(many).collect(),
            (None, true) => {
                return McpCallToolResult::error(
                    "Argument 'relative_path' (or 'relative_paths') is required",
                );
            }
        };
        if relative_paths.len() > MAX_NOTES {
            return McpCallToolResult::error(format!(
                "At most {MAX_NOTES} notes per call (received {}); to search across the vault use full_text_search",
                relative_paths.len()
            ));
        }
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return McpCallToolResult::error("Argument 'query' is required"),
        };
        let flag = |k: &str| args.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
        let counts_only = flag("counts_only");
        let max = args
            .get("max_matches")
            .and_then(|v| v.as_u64())
            .map(|m| m as usize)
            .unwrap_or(usize::MAX);
        // Sin `relative_paths`, la respuesta mantiene la forma de siempre (una sola nota)
        let as_list = !string_list(args.get("relative_paths")).is_empty();

        let result = match SearchInNoteUseCase::new(&self.note_repository).execute(SearchInNoteQuery {
            vault_path: self.vault(),
            relative_paths,
            query: query.to_string(),
            literal: flag("literal"),
            context_lines: args.get("context_lines").and_then(|v| v.as_u64()).unwrap_or(2) as usize,
        }) {
            Ok(r) => r,
            Err(e) => return McpCallToolResult::error(e),
        };
        let several = result.alternatives.len() > 1;
        let note_json = |rel: &str, m: &crate::application::dto::note::search_in_note_query::NoteMatches| {
            let counts: serde_json::Map<String, Value> = m
                .counts
                .iter()
                .map(|c| {
                    (c.alternative.clone(), json!({ "lines": c.lines, "occurrences": c.occurrences }))
                })
                .collect();
            let mut out = json!({
                "relative_path": rel,
                "total_matches": m.matches.len(),
                "counts": counts,
            });
            if !counts_only {
                let matches: Vec<Value> = m
                    .matches
                    .iter()
                    .take(max)
                    .map(|l| {
                        let mut line = json!({
                            "line_number": l.line_number,
                            "matched_line": l.matched_line,
                            "context": l.context,
                        });
                        if several {
                            line["matched"] = json!(l.matched);
                        }
                        line
                    })
                    .collect();
                out["returned_matches"] = json!(matches.len());
                out["matches"] = json!(matches);
            }
            out
        };

        if !as_list {
            let note = &result.notes[0];
            return match &note.outcome {
                Ok(m) => {
                    let mut payload = note_json(&note.relative_path, m);
                    if several {
                        payload["alternatives"] = json!(result.alternatives);
                    }
                    to_text(&payload, "Failed to serialize: ")
                }
                Err(e) => McpCallToolResult::error(e.clone()),
            };
        }

        let all_failed = result.notes.iter().all(|n| n.outcome.is_err());
        let files: Vec<Value> = result
            .notes
            .iter()
            .map(|n| match &n.outcome {
                Ok(m) => note_json(&n.relative_path, m),
                Err(e) => {
                    let error = match self.location_hint(&n.relative_path) {
                        Some(hint) => format!("{e}. {hint}"),
                        None => e.clone(),
                    };
                    json!({ "relative_path": n.relative_path, "error": error })
                }
            })
            .collect();
        let total: usize = result
            .notes
            .iter()
            .filter_map(|n| n.outcome.as_ref().ok())
            .map(|m| m.matches.len())
            .sum();
        let mut payload = json!({ "total_matches": total, "files": files });
        if several {
            payload["alternatives"] = json!(result.alternatives);
        }
        let mut out = to_text(&payload, "Failed to serialize: ");
        out.is_error = all_failed;
        out
    }

    /// XML (`mxfile`) del diagrama de draw.io de `rel`: un `.drawio`, o un PNG/SVG con el diagrama
    /// dentro. `Ok(None)` si no es de draw.io. Con `revision`, tal como estaba en ese commit.
    fn drawio_source(&self, rel: &str, revision: Option<&str>) -> Result<Option<String>, McpCallToolResult> {
        use crate::infrastructure::search::drawio_extractor::{png_mxfile, svg_mxfile};
        let lower = rel.to_lowercase();
        let kind = if lower.ends_with(".drawio") {
            "xml"
        } else if lower.ends_with(".png") {
            "png"
        } else if lower.ends_with(".svg") {
            "svg"
        } else {
            return Ok(None);
        };
        let bytes: Vec<u8> = match revision {
            Some(rev) => {
                let git = self.git()?;
                let r = git.get_file_revision(&self.vault(), rel, rev).map_err(McpCallToolResult::error)?;
                if !r.exists {
                    return Err(McpCallToolResult::error(format!("'{rel}' did not exist at {rev}")));
                }
                match (r.content, r.data_url) {
                    (Some(text), _) if !r.is_binary => text.into_bytes(),
                    (_, Some(url)) => {
                        use base64::Engine;
                        let data = url.split_once(',').map(|(_, d)| d).unwrap_or_default();
                        base64::engine::general_purpose::STANDARD.decode(data).map_err(|e| McpCallToolResult::error(e.to_string()))?
                    }
                    _ => return Ok(None),
                }
            }
            None => {
                let rel_path = crate::domain::note::NoteRelativePath::new(rel).map_err(McpCallToolResult::error)?;
                std::fs::read(self.vault().join(rel_path.as_str()))
                    .map_err(|e| McpCallToolResult::error(format!("Could not read '{rel}': {e}")))?
            }
        };
        Ok(match kind {
            "xml" => Some(String::from_utf8_lossy(&bytes).into_owned()),
            "png" => png_mxfile(&bytes),
            _ => svg_mxfile(&bytes),
        })
    }

    fn execute_read_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        // Modelo ArchiMate: el índice son sus carpetas y vistas, no encabezados Markdown
        let wants_outline = args.get("mode").and_then(|v| v.as_str()) == Some("outline_only")
            || args.get("outline_only").and_then(|v| v.as_bool()).unwrap_or(false);
        if wants_outline && rel_path_str.to_lowercase().ends_with(".archimate") {
            return match self.archimate_model(rel_path_str) {
                Err(e) => e,
                Ok(model) => to_text(
                    &json!({
                        "relative_path": rel_path_str,
                        "total_lines": model.lines(),
                        "toc": model.outline(),
                        "note": "ArchiMate model: folders and views with their line numbers. To inspect an element (properties, relations, and views where it appears), use inspect_archimate_element; to read a line range, use read_note with mode \"chunk\".",
                    }),
                    "",
                ),
            };
        }
        // Diagramas de draw.io (también dentro de un PNG/SVG) e imágenes: no son texto que leer
        // por líneas. Se devuelve el texto del diagrama y se indica qué herramienta usar.
        let lower = rel_path_str.to_lowercase();
        if lower.ends_with(".drawio") || lower.ends_with(".png") || lower.ends_with(".svg") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".webp") || lower.ends_with(".gif") {
            return match self.drawio_source(rel_path_str, None) {
                Err(e) => e,
                Ok(Some(xml)) => to_text(
                    &json!({
                        "relative_path": rel_path_str,
                        "diagram": {
                            "type": "drawio",
                            "text": crate::infrastructure::search::drawio_extractor::mxfile_text(&xml),
                            "render_tool": "render_diagram",
                            "structure_tool": "extract_note_diagrams",
                        },
                        "note": "draw.io diagram: `text` contains labels from its shapes and pages. Use render_diagram to view it as an image, or extract_note_diagrams for its containers and connections.",
                    }),
                    "",
                ),
                Ok(None) if lower.ends_with(".svg") => McpCallToolResult::error(
                    "This is an SVG image: use render_diagram with the same relative_path to view it.",
                ),
                Ok(None) => McpCallToolResult::error(
                    "This is an image, not a text note: use render_diagram with the same relative_path to view it.",
                ),
            };
        }
        let mut mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");
        let start_line = args.get("start_line").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let line_count = (args
            .get("line_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(150) as usize)
            .min(500);
        let section_heading = args
            .get("section")
            .or_else(|| args.get("section_heading"))
            .and_then(|v| v.as_str());
        let outline_only = args
            .get("outline_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let include_line_numbers = args
            .get("include_line_numbers")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Si se especificó outline_only o section_heading, inferir el modo automáticamente si mode es el predeterminado
        if outline_only || mode == "outline_only" {
            mode = "metadata_only";
        } else if section_heading.is_some() && mode == "full" {
            mode = "section";
        }

        let details =
            match ReadNoteDetailsUseCase::new(&self.note_repository).execute(ReadNoteDetailsQuery {
                vault_path: self.vault(),
                relative_path: rel_path_str.to_string(),
            }) {
                Ok(d) => d,
                Err(e) => return McpCallToolResult::error(e),
            };
        let note = &details.note;
        let lines: Vec<&str> = note.content.lines().collect();
        let total_lines = details.total_lines;
        let toc: Vec<Value> = details
            .toc
            .iter()
            .map(|h| json!({ "level": h.level, "text": h.text, "line": h.line }))
            .collect();
        let detected_blocks =
            json!({ "mermaid_count": details.mermaid_count, "d2_count": details.d2_count });
        let render = |offset: usize, selected: &[&str]| -> String {
            if include_line_numbers {
                selected
                    .iter()
                    .enumerate()
                    .map(|(i, l)| format!("{}: {}", offset + i + 1, l))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                selected.join("\n")
            }
        };

        let payload = match mode {
            "metadata_only" => json!({
                "relative_path": note.relative_path.as_str(),
                "title": note.title,
                "content_hash": details.content_hash,
                "total_lines": total_lines,
                "estimated_tokens": details.estimated_tokens,
                "okf_metadata": note.frontmatter,
                "toc": toc,
                "detected_blocks": detected_blocks,
            }),
            "section" => {
                let target_heading = match section_heading {
                    Some(h) => h.trim().trim_start_matches('#').trim().to_lowercase(),
                    None => {
                        return McpCallToolResult::error(
                            "Argument 'section' or 'section_heading' is required for mode='section'",
                        );
                    }
                };
                let (start_idx, section_level) = match NoteContent::find_heading(
                    &lines,
                    &target_heading,
                ) {
                    Some(found) => found,
                    None => {
                        let available_headings: Vec<String> =
                            details.toc.iter().map(|h| h.text.clone()).collect();
                        return McpCallToolResult::error(format!(
                            "Section '{}' not found in note. Available sections: {:?}",
                            target_heading, available_headings
                        ));
                    }
                };
                let end_idx = NoteContent::section_end(&lines, start_idx, section_level);
                json!({
                    "relative_path": note.relative_path.as_str(),
                    "title": note.title,
                    "content_hash": details.content_hash,
                    "total_lines": total_lines,
                    "returned_lines": { "start": start_idx + 1, "end": end_idx },
                    "okf_metadata": note.frontmatter,
                    "toc": toc,
                    "content": render(start_idx, &lines[start_idx..end_idx]),
                })
            }
            "chunk" => {
                let start_idx = start_line.saturating_sub(1).min(total_lines);
                let end_idx = (start_idx + line_count).min(total_lines);
                let selected: &[&str] = if start_idx < end_idx {
                    &lines[start_idx..end_idx]
                } else {
                    &[]
                };
                json!({
                    "relative_path": note.relative_path.as_str(),
                    "title": note.title,
                    "content_hash": details.content_hash,
                    "total_lines": total_lines,
                    "returned_lines": { "start": start_idx + 1, "end": end_idx },
                    "has_more": end_idx < total_lines,
                    "okf_metadata": note.frontmatter,
                    "toc": toc,
                    "content": render(start_idx, selected),
                })
            }
            _ => {
                // mode = "full"
                let content = if include_line_numbers {
                    render(0, &lines)
                } else {
                    note.content.clone()
                };
                json!({
                    "relative_path": note.relative_path.as_str(),
                    "title": note.title,
                    "content_hash": details.content_hash,
                    "total_lines": total_lines,
                    "estimated_tokens": details.estimated_tokens,
                    "okf_metadata": note.frontmatter,
                    "toc": toc,
                    "detected_blocks": detected_blocks,
                    "content": content,
                })
            }
        };
        let mut payload = payload;
        // Marca de diagrama: el agente puede pedirlo como imagen con `render_diagram`
        if let (Some(kind), Some(obj)) = (diagram_kind(rel_path_str), payload.as_object_mut()) {
            obj.insert(
                "diagram".into(),
                json!({
                    "kind": kind,
                    "render_tool": "render_diagram",
                    "renderable": self.diagram_renderer.is_some(),
                    "hint": if self.diagram_renderer.is_some() {
                        "This is a diagram: call render_diagram with this relative_path to view it as a PNG image."
                    } else {
                        "This is a diagram; rendering it as an image requires the Synapse window to be open (instance MCP)."
                    },
                }),
            );
        }
        to_text(&payload, "")
    }

    fn git(&self) -> Result<&Arc<G>, McpCallToolResult> {
        self.git_service
            .as_ref()
            .ok_or_else(|| McpCallToolResult::error("Git is not available in this mode"))
    }

    fn execute_git_file_history(&self, args: &Value) -> McpCallToolResult {
        let git = match self.git() {
            Ok(g) => g,
            Err(e) => return e,
        };
        let Some(rel) = args.get("relative_path").and_then(|v| v.as_str()) else {
            return McpCallToolResult::error("Argument 'relative_path' is required");
        };
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .clamp(1, 200) as usize;
        match git.get_file_log(&self.vault(), rel, Some(limit)) {
            Ok(log) if log.commits.is_empty() => {
                McpCallToolResult::error(format!("'{rel}' has no commits in Git history"))
            }
            Ok(log) => to_text(
                &json!({
                    "relative_path": rel,
                    "commits": log.commits.iter().map(|c| json!({
                        "hash": c.commit_hash,
                        "short_hash": c.short_hash,
                        "date": c.date,
                        "author": c.author_name,
                        "message": c.message,
                        "tags": c.tags,
                    })).collect::<Vec<_>>(),
                }),
                "",
            ),
            Err(e) => McpCallToolResult::error(e),
        }
    }

    fn execute_git_diff_revisions(&self, args: &Value) -> McpCallToolResult {
        let git = match self.git() {
            Ok(g) => g,
            Err(e) => return e,
        };
        let text = |k: &str| {
            args.get(k)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        };
        let (Some(rel), Some(from)) = (text("relative_path"), text("from")) else {
            return McpCallToolResult::error(
                "Arguments 'relative_path' and 'from' are required",
            );
        };
        let to = text("to");
        let vault = self.vault();
        // De `from` a `to` (o a la versión local)
        let (old, new, old_path, new_path) = match to {
            None => match git.diff(&vault, rel, false, Some(from)) {
                Ok(d) => (
                    d.old_content,
                    d.new_content,
                    from.to_string(),
                    "local".to_string(),
                ),
                Err(e) => return McpCallToolResult::error(e),
            },
            Some(to) => {
                let read = |rev: &str| git.get_file_revision(&vault, rel, rev);
                match (read(from), read(to)) {
                    (Ok(a), Ok(b)) => {
                        if !a.exists && !b.exists {
                            return McpCallToolResult::error(format!(
                                "'{rel}' did not exist at {from} or {to}"
                            ));
                        }
                        if a.is_binary || b.is_binary {
                            return McpCallToolResult::error(format!(
                                "'{rel}' is binary: compare visually with render_diagram and `revision` on each version"
                            ));
                        }
                        (
                            a.content.unwrap_or_default(),
                            b.content.unwrap_or_default(),
                            from.to_string(),
                            to.to_string(),
                        )
                    }
                    (Err(e), _) | (_, Err(e)) => return McpCallToolResult::error(e),
                }
            }
        };
        let diff = similar::TextDiff::from_lines(&old, &new);
        let (mut added, mut removed) = (0usize, 0usize);
        for change in diff.iter_all_changes() {
            match change.tag() {
                similar::ChangeTag::Insert => added += 1,
                similar::ChangeTag::Delete => removed += 1,
                similar::ChangeTag::Equal => {}
            }
        }
        let unified = diff
            .unified_diff()
            .header(
                &format!("{rel} @ {old_path}"),
                &format!("{rel} @ {new_path}"),
            )
            .to_string();
        let lines: Vec<&str> = unified.lines().collect();
        let truncated = lines.len() > GIT_MAX_LINES;
        to_text(
            &json!({
                "relative_path": rel,
                "from": old_path,
                "to": new_path,
                "lines_added": added,
                "lines_removed": removed,
                "identical": added == 0 && removed == 0,
                "truncated": truncated,
                "diff": lines[..lines.len().min(GIT_MAX_LINES)].join("\n"),
            }),
            "",
        )
    }

    fn execute_git_read_revision(&self, args: &Value) -> McpCallToolResult {
        let git = match self.git() {
            Ok(g) => g,
            Err(e) => return e,
        };
        let text = |k: &str| {
            args.get(k)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        };
        let (Some(rel), Some(rev)) = (text("relative_path"), text("revision")) else {
            return McpCallToolResult::error(
                "Arguments 'relative_path' and 'revision' are required",
            );
        };
        let start = args
            .get("start_line")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .max(1) as usize;
        let count = (args
            .get("line_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(300) as usize)
            .clamp(1, GIT_MAX_LINES);
        match git.get_file_revision(&self.vault(), rel, rev) {
            Ok(r) if !r.exists => McpCallToolResult::error(format!("'{rel}' did not exist at {rev}")),
            Ok(r) if r.is_binary => McpCallToolResult::error(format!(
                "'{rel}' is binary at {rev}: if it is a diagram, use render_diagram with `revision`"
            )),
            Ok(r) => {
                let content = r.content.unwrap_or_default();
                let lines: Vec<&str> = content.lines().collect();
                let from = (start - 1).min(lines.len());
                let to = (from + count).min(lines.len());
                to_text(
                    &json!({
                        "relative_path": rel,
                        "revision": rev,
                        "path_at_revision": r.path,
                        "total_lines": lines.len(),
                        "start_line": from + 1,
                        "end_line": to,
                        "content": lines[from..to].join("\n"),
                    }),
                    "",
                )
            }
            Err(e) => McpCallToolResult::error(e),
        }
    }

    fn execute_list_vaults(&self) -> McpCallToolResult {
        let Some(catalog) = &self.vault_catalog else {
            return McpCallToolResult::error(
                "Querying other vaults is not available in this mode",
            );
        };
        let here = self.vault().canonicalize().unwrap_or_else(|_| self.vault());
        let access = read_vault_access(&here);
        let list: Vec<Value> = catalog
            .list()
            .into_iter()
            .map(|v| {
                let key = v.path.to_string_lossy().into_owned();
                json!({
                    "name": v.name,
                    "path": key,
                    "open": v.open,
                    "is_this": v.path == here,
                    "allowed": access.get(&key).copied(),
                })
            })
            .collect();
        to_text(&list, "")
    }

    /// Ejecuta una herramienta de lectura sobre otra bóveda, con el permiso de esta para consultarla.
    fn call_in_other_vault(&self, name: &str, vault: &str, args: Value) -> McpCallToolResult {
        let here = self.vault().canonicalize().unwrap_or_else(|_| self.vault());
        let Some(catalog) = &self.vault_catalog else {
            return McpCallToolResult::error(
                "Querying other vaults is not available in this mode",
            );
        };
        let vaults = catalog.list();
        let target = match resolve_vault(&vaults, vault) {
            Ok(v) => v,
            Err(e) => return McpCallToolResult::error(e),
        };
        if target.path == here {
            return self.call_tool(name, Some(args));
        }
        if name == "render_diagram" {
            return self.render_in_other_vault(&here, &target, &args);
        }
        if !CROSS_VAULT_TOOLS.contains(&name) {
            return McpCallToolResult::error(format!(
                "'{name}' cannot be used on another vault: only read tools are allowed ({})",
                CROSS_VAULT_TOOLS.join(", ")
            ));
        }
        if let Err(e) = self.ensure_vault_access(&here, &target) {
            return McpCallToolResult::error(e);
        }
        let fts = crate::infrastructure::repository::tantivy::full_text_index_impl::FullTextIndexImpl::open_read_only(
            &target.path.join(".synapse").join("fts"),
        )
        .ok()
        .map(Arc::new);
        let other = McpToolHandler::new(
            target.path.clone(),
            crate::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl::new(
            ),
            fts,
            Some(Arc::new(
                crate::infrastructure::repository::git::version_control_impl::VersionControlImpl,
            )),
            Arc::clone(&self.vault_item_repository),
            Arc::clone(&self.pdf_document_repository),
        );
        let mut result = other.call_tool(name, Some(args));
        result.content.insert(
            0,
            McpContentItem::text(format!("📚 Vault «{}» (read-only)", target.name)),
        );
        result
    }

    /// El diagrama (o la imagen, como un `.drawio.png`) tal como era en un commit de Git.
    fn render_at_revision(&self, rel: &str, rev: &str) -> McpCallToolResult {
        let git = match self.git() {
            Ok(g) => g,
            Err(e) => return e,
        };
        let r = match git.get_file_revision(&self.vault(), rel, rev) {
            Ok(r) if r.exists => r,
            Ok(_) => return McpCallToolResult::error(format!("'{rel}' did not exist at {rev}")),
            Err(e) => return McpCallToolResult::error(e),
        };
        let header = format!("«{rel}» at {rev} (file in commit: {})", r.path);
        // Imágenes (png, jpg, svg… incluido un `.drawio.png`): se devuelven tal cual
        if let Some(data_url) = r
            .data_url
            .as_deref()
            .filter(|u| u.starts_with("data:image/"))
            && (r.is_binary || diagram_kind(rel).is_none())
                && let Some((meta, data)) = data_url.split_once(',') {
                    let mime = meta
                        .trim_start_matches("data:")
                        .trim_end_matches(";base64")
                        .to_string();
                    return McpCallToolResult {
                        content: vec![
                            McpContentItem::text(format!("Image {header}.")),
                            McpContentItem::image(data.to_string(), mime),
                        ],
                        is_error: false,
                    };
                }
        let Some(kind) = diagram_kind(rel) else {
            return McpCallToolResult::error(format!(
                "'{rel}' is not an image or diagram that Synapse can render; read it with git_read_revision"
            ));
        };
        let Some(renderer) = &self.diagram_renderer else {
            return McpCallToolResult::error(
                "Rendering diagrams requires the Synapse window to be open",
            );
        };
        match renderer.render(rel, r.content.as_deref()) {
            Ok(img) => McpCallToolResult {
                content: vec![
                    McpContentItem::text(format!(
                        "{kind} diagram {header}, rendered as image."
                    )),
                    McpContentItem::image(img.data_base64, img.mime_type),
                ],
                is_error: false,
            },
            Err(e) => McpCallToolResult::error(format!("Could not render '{rel}' at {rev}: {e}")),
        }
    }

    /// Dibuja un diagrama de otra bóveda con los motores de esta ventana (lee su archivo con permiso).
    fn render_in_other_vault(
        &self,
        here: &std::path::Path,
        target: &VaultRef,
        args: &Value,
    ) -> McpCallToolResult {
        let Some(rel) = args.get("relative_path").and_then(|v| v.as_str()) else {
            return McpCallToolResult::error("Argument 'relative_path' is required");
        };
        let Some(kind) = diagram_kind(rel) else {
            return McpCallToolResult::error(format!(
                "'{rel}' is not a diagram that Synapse can render"
            ));
        };
        let Some(renderer) = &self.diagram_renderer else {
            return McpCallToolResult::error(
                "Rendering diagrams requires the Synapse window to be open",
            );
        };
        if let Err(e) = self.ensure_vault_access(here, target) {
            return McpCallToolResult::error(e);
        }
        let file = target.path.join(rel);
        let inside = file
            .canonicalize()
            .map(|f| f.starts_with(&target.path))
            .unwrap_or(false);
        if !inside {
            return McpCallToolResult::error(format!(
                "File '{rel}' does not exist in «{}»",
                target.name
            ));
        }
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => return McpCallToolResult::error(format!("Could not read '{rel}': {e}")),
        };
        match renderer.render(rel, Some(&content)) {
            Ok(img) => McpCallToolResult {
                content: vec![
                    McpContentItem::text(format!(
                        "📚 Vault «{}» · {kind} diagram «{rel}» rendered as image.",
                        target.name
                    )),
                    McpContentItem::image(img.data_base64, img.mime_type),
                ],
                is_error: false,
            },
            Err(e) => McpCallToolResult::error(format!("Could not render '{rel}': {e}")),
        }
    }

    /// Permiso de esta bóveda para consultar `target`: guardado, o se pregunta al usuario la primera vez.
    fn ensure_vault_access(&self, here: &std::path::Path, target: &VaultRef) -> Result<(), String> {
        let key = target.path.to_string_lossy().into_owned();
        let mut access = read_vault_access(here);
        match access.get(&key) {
            Some(true) => return Ok(()),
            Some(false) => {
                return Err(format!(
                    "The user did not allow this vault to query «{}».",
                    target.name
                ));
            }
            None => {}
        }
        let Some(prompter) = &self.prompter else {
            return Err(format!(
                "Querying «{}» requires user permission; open it from Synapse with the window open.",
                target.name
            ));
        };
        let here_name = here
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let allowed = prompter.confirm(
            "Query another vault",
            &format!(
                "An agent from «{here_name}» wants to read notes in «{}» (read-only). Allow?",
                target.name
            ),
        )?;
        access.insert(key, allowed);
        write_vault_access(here, &access);
        if allowed {
            Ok(())
        } else {
            Err(format!(
                "The user did not allow querying «{}».",
                target.name
            ))
        }
    }

    fn conversations(&self) -> Result<&Arc<dyn ConversationHub>, McpCallToolResult> {
        self.conversation_hub.as_ref().ok_or_else(|| {
            McpCallToolResult::error(
                "Messaging other conversations requires the Synapse window to be open (instance MCP)",
            )
        })
    }

    fn execute_list_conversations(&self) -> McpCallToolResult {
        let hub = match self.conversations() {
            Ok(h) => h,
            Err(e) => return e,
        };
        match hub.list(self.caller.as_deref()) {
            Ok(list) => to_text(&list, ""),
            Err(e) => McpCallToolResult::error(e),
        }
    }

    fn execute_send_to_conversation(&self, args: &Value) -> McpCallToolResult {
        let hub = match self.conversations() {
            Ok(h) => h,
            Err(e) => return e,
        };
        let Some(to) = args
            .get("to")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
        else {
            return McpCallToolResult::error("Argument 'to' is required");
        };
        let Some(message) = args
            .get("message")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
        else {
            return McpCallToolResult::error("Argument 'message' is required");
        };
        let expects_reply = args
            .get("expects_reply")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        match hub.send(self.caller.as_deref(), to, message, expects_reply) {
            Ok(result) => to_text(&result, ""),
            Err(e) => McpCallToolResult::error(e),
        }
    }

    /// Dibuja un diagrama de la bóveda como imagen PNG, a través de la ventana de Synapse.
    fn execute_render_diagram(&self, args: &Value) -> McpCallToolResult {
        let Some(rel) = args.get("relative_path").and_then(|v| v.as_str()) else {
            return McpCallToolResult::error("Argument 'relative_path' is required");
        };
        if let Some(rev) = args
            .get("revision")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return self.render_at_revision(rel, rev);
        }
        // Imágenes (incluido un `.drawio.png` o `.drawio.svg`): se devuelven tal cual
        let lower = rel.to_lowercase();
        let image_mime = [("png", "image/png"), ("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("gif", "image/gif"), ("webp", "image/webp"), ("svg", "image/svg+xml")]
            .into_iter()
            .find(|(ext, _)| lower.ends_with(&format!(".{ext}")))
            .map(|(_, mime)| mime);
        if let Some(mime) = image_mime {
            let Ok(rel_path) = crate::domain::note::NoteRelativePath::new(rel) else {
                return McpCallToolResult::error(format!("Invalid path: '{rel}'"));
            };
            return match std::fs::read(self.vault().join(rel_path.as_str())) {
                Ok(bytes) => {
                    use base64::Engine;
                    McpCallToolResult {
                        content: vec![
                            McpContentItem::text(format!("Image «{rel}».")),
                            McpContentItem::image(base64::engine::general_purpose::STANDARD.encode(&bytes), mime.to_string()),
                        ],
                        is_error: false,
                    }
                }
                Err(e) => McpCallToolResult::error(format!("Could not read '{rel}': {e}")),
            };
        }
        let Some(kind) = diagram_kind(rel) else {
            return McpCallToolResult::error(format!(
                "'{rel}' is not a diagram that Synapse can render (Excalidraw, Mermaid, D2, or PlantUML)"
            ));
        };
        if !self.vault().join(rel).is_file() {
            return McpCallToolResult::error(format!("File '{rel}' does not exist in the vault"));
        }
        let Some(renderer) = &self.diagram_renderer else {
            return McpCallToolResult::error(
                "Rendering diagrams requires the Synapse window to be open (use instance MCP)",
            );
        };
        match renderer.render(rel, None) {
            Ok(img) => McpCallToolResult {
                content: vec![
                    McpContentItem::text(format!("{kind} diagram «{rel}» rendered as image.")),
                    McpContentItem::image(img.data_base64, img.mime_type),
                ],
                is_error: false,
            },
            Err(e) => McpCallToolResult::error(format!("Could not render '{rel}': {e}")),
        }
    }

    fn execute_write_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return McpCallToolResult::error("Argument 'content' is required"),
        };
        let command = WriteNoteCommand {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
            content: content.to_string(),
            title: args
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tags: args
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|_| string_list(args.get("tags"))),
            aliases: args
                .get("aliases")
                .and_then(|v| v.as_array())
                .map(|_| string_list(args.get("aliases"))),
            expected_hash: args
                .get("expected_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            overwrite: args
                .get("overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            allow_large_overwrite: args
                .get("allow_large_overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            dry_run: args
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };
        // Antes de escribir: guardar la nota crea las carpetas que falten.
        let missing_folder = Path::new(rel_path_str)
            .parent()
            .filter(|p| !p.as_os_str().is_empty() && !self.vault().join(p).exists())
            .map(|p| p.to_string_lossy().to_string());
        let mut payload = match WriteNoteUseCase::new(&self.note_repository).execute(command) {
            Err(e) => return McpCallToolResult::error(e),
            Ok(WriteNoteOutcome::Preview {
                created,
                old_content,
                new_content,
            }) => json!({
                "dry_run": true,
                "created": created,
                "relative_path": rel_path_str,
                "unified_diff": generate_unified_diff(&old_content, &new_content, rel_path_str),
            }),
            Ok(WriteNoteOutcome::Saved { created, new_hash }) => json!({
                "status": "success",
                "created": created,
                "relative_path": rel_path_str,
                "new_content_hash": new_hash,
                "message": format!(
                    "Note '{}' {} successfully.",
                    rel_path_str,
                    if created { "created" } else { "overwritten" }
                ),
            }),
        };
        if let Some(folder) = missing_folder {
            payload["warning"] = json!(format!(
                "Folder '{}' did not exist in the vault. Check that the name is correct.",
                folder
            ));
        }
        to_text(&payload, "")
    }

    fn execute_patch_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        let old_text = match args.get("old_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return McpCallToolResult::error("Argument 'old_text' is required"),
        };
        let new_text = match args.get("new_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return McpCallToolResult::error("Argument 'new_text' is required"),
        };
        let command = PatchNoteCommand {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
            old_text: old_text.to_string(),
            new_text: new_text.to_string(),
            start_line: args
                .get("start_line")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            expected_hash: args
                .get("expected_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            dry_run: args
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };
        let outcome = match PatchNoteUseCase::new(&self.note_repository).execute(command) {
            Ok(o) => o,
            Err(e) => return McpCallToolResult::error(e),
        };
        let unified_diff =
            generate_unified_diff(&outcome.old_content, &outcome.new_content, rel_path_str);
        let payload = match outcome.new_hash {
            None => json!({
                "dry_run": true,
                "relative_path": rel_path_str,
                "unified_diff": unified_diff,
            }),
            Some(new_hash) => json!({
                "status": "success",
                "relative_path": rel_path_str,
                "new_content_hash": new_hash,
                "unified_diff": unified_diff,
            }),
        };
        to_text(&payload, "")
    }

    fn execute_append_to_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        let content_to_append = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return McpCallToolResult::error("Argument 'content' is required"),
        };
        let command = AppendToNoteCommand {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
            content: content_to_append.to_string(),
            section_heading: args
                .get("section_heading")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            position: args
                .get("position")
                .and_then(|v| v.as_str())
                .unwrap_or("end_of_section")
                .to_string(),
        };
        match AppendToNoteUseCase::new(&self.note_repository).execute(command) {
            Ok(new_hash) => to_text(
                &json!({
                    "status": "success",
                    "relative_path": rel_path_str,
                    "new_content_hash": new_hash,
                }),
                "",
            ),
            Err(e) => McpCallToolResult::error(e),
        }
    }

    fn execute_update_note_metadata(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        let optional_list = |key: &str| {
            args.get(key)
                .and_then(|v| v.as_array())
                .map(|_| string_list(args.get(key)))
        };
        let command = UpdateNoteMetadataCommand {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
            title: args
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            add_tags: optional_list("add_tags"),
            remove_tags: optional_list("remove_tags"),
            aliases: optional_list("aliases"),
        };
        match UpdateNoteMetadataUseCase::new(&self.note_repository).execute(command) {
            Ok((fm, new_hash)) => to_text(
                &json!({
                    "status": "success",
                    "relative_path": rel_path_str,
                    "updated_metadata": fm,
                    "new_content_hash": new_hash,
                }),
                "",
            ),
            Err(e) => McpCallToolResult::error(e),
        }
    }

    fn execute_list_folder_contents_tool(&self, args: &Value) -> McpCallToolResult {
        let folder = args
            .get("folder_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let include_notes = args
            .get("include_notes")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let include_folders = args
            .get("include_folders")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50)
            .clamp(1, 200) as usize;
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        self.execute_list_folder_contents(
            folder,
            include_notes,
            include_folders,
            recursive,
            None,
            Some(limit),
            Some(offset),
        )
    }

    fn execute_list_sibling_notes(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        if rel_path_str.is_empty() {
            return McpCallToolResult::error("Argument 'relative_path' cannot be empty");
        }

        let include_self = args
            .get("include_self")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let include_folders = args
            .get("include_folders")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let parent_folder = match rel_path_str.rsplit_once('/') {
            Some((dir, _)) => dir,
            None => "",
        };
        let exclude_path = if include_self {
            None
        } else {
            Some(rel_path_str.as_str())
        };

        self.execute_list_folder_contents(
            parent_folder,
            true,
            include_folders,
            false,
            exclude_path,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_list_folder_contents(
        &self,
        folder: &str,
        include_notes: bool,
        include_folders: bool,
        recursive: bool,
        exclude_note_path: Option<&str>,
        limit_opt: Option<usize>,
        offset_opt: Option<usize>,
    ) -> McpCallToolResult {
        let clean_folder = folder.trim_matches('/').replace('\\', "/");
        let listing = match ListFolderContentsUseCase::new(
            &self.note_repository,
            &*self.vault_item_repository,
        )
        .execute(ListFolderContentsQuery {
            vault_path: self.vault(),
            folder: clean_folder.clone(),
            include_notes,
            include_folders,
            recursive,
            exclude_note_path: exclude_note_path.map(|s| s.to_string()),
        }) {
            Ok(l) => l,
            Err(e) => return McpCallToolResult::error(e),
        };

        let folders_out: Vec<_> = listing
            .folders
            .into_iter()
            .map(|f| json!({ "name": f.name, "relative_path": f.relative_path }))
            .collect();
        let notes_out: Vec<_> = listing
            .notes
            .into_iter()
            .map(|n| match n.line_count {
                Some(line_count) => json!({
                    "name": n.name,
                    "title": n.title,
                    "relative_path": n.relative_path,
                    "line_count": line_count,
                }),
                None => json!({
                    "name": n.name,
                    "relative_path": n.relative_path,
                    "title": n.title,
                }),
            })
            .collect();

        let total_notes = notes_out.len();
        let offset = offset_opt.unwrap_or(0);
        let limit = limit_opt.unwrap_or(50);
        let has_more = offset + limit < total_notes;
        let next_offset = if has_more { Some(offset + limit) } else { None };
        let paged_notes: Vec<_> = notes_out.into_iter().skip(offset).take(limit).collect();

        let payload = json!({
            "folder_path": clean_folder,
            "total_folders": folders_out.len(),
            "folders_count": folders_out.len(),
            "folders": folders_out,
            "total_notes": total_notes,
            "notes_count": paged_notes.len(),
            "offset": offset,
            "limit": limit,
            "has_more": has_more,
            "next_offset": next_offset,
            "notes": paged_notes,
        });
        to_text(&payload, "Failed to serialize contents: ")
    }

    fn execute_move_notes_tool(&self, args: &Value) -> McpCallToolResult {
        let notes_val = match args.get("notes").and_then(|v| v.as_array()) {
            Some(n) => n,
            None => {
                return McpCallToolResult::error(
                    "Argument 'notes' must be a list of paths",
                );
            }
        };
        let target_folder = match args.get("target_folder").and_then(|v| v.as_str()) {
            Some(tf) => tf,
            None => return McpCallToolResult::error("Argument 'target_folder' is required"),
        };
        let command = MoveNotesCommand {
            vault_path: self.vault(),
            notes: notes_val
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            target_folder: target_folder.to_string(),
            create_target_folder: args
                .get("create_target_folder")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            overwrite: args
                .get("overwrite")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            update_wikilinks: args
                .get("update_wikilinks")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };
        let result =
            match MoveNotesUseCase::new(&self.note_repository, &*self.vault_item_repository)
                .execute(command)
            {
                Ok(r) => r,
                Err(e) => return McpCallToolResult::error(e),
            };
        let moved: Vec<_> = result
            .moved
            .into_iter()
            .map(|(source, destination)| json!({ "source": source, "destination": destination }))
            .collect();
        let errors: Vec<_> = result
            .errors
            .into_iter()
            .map(|(source, error)| json!({ "source": source, "error": error }))
            .collect();
        let payload = json!({
            "moved_count": moved.len(),
            "moved": moved,
            "error_count": errors.len(),
            "errors_count": errors.len(),
            "errors": errors,
        });
        to_text(&payload, "Failed to serialize move_notes: ")
    }

    fn execute_delete_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        if rel_path_str.is_empty() {
            return McpCallToolResult::error("Argument 'relative_path' cannot be empty");
        }
        let command = DeleteNoteCommand {
            vault_path: self.vault(),
            relative_path: rel_path_str.clone(),
            expected_hash: args
                .get("expected_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
        match DeleteNoteUseCase::new(&self.note_repository).execute(command) {
            Ok(()) => to_text(
                &json!({
                    "status": "success",
                    "deleted_path": rel_path_str,
                    "message": "Note deleted successfully"
                }),
                "",
            ),
            Err(e) => McpCallToolResult::error(e),
        }
    }

    fn execute_get_backlinks_and_relations(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        let relations = match GetNoteRelationsUseCase::new(&self.note_repository).execute(
            GetNoteRelationsQuery {
                vault_path: self.vault(),
                relative_path: rel_path_str.to_string(),
            },
        ) {
            Ok(r) => r,
            Err(e) => return McpCallToolResult::error(e),
        };
        let backlinks: Vec<_> = relations
            .backlinks
            .into_iter()
            .map(|b| json!({ "source_path": b.source_path, "title": b.title, "context_snippet": b.context_snippet }))
            .collect();
        let outlinks: Vec<_> = relations
            .outlinks
            .into_iter()
            .map(|o| {
                let mut v = json!({ "target": o.target, "alias": o.alias, "is_embed": o.is_embed, "is_resolved": o.is_resolved, "resolved_path": o.resolved_path });
                // Varios archivos con ese nombre: no se elige ninguno; conviene escribir la ruta completa
                if !o.ambiguous.is_empty() {
                    v["ambiguous"] = json!(o.ambiguous);
                }
                v
            })
            .collect();
        let neighbors_json: Vec<_> = relations
            .neighbors
            .into_iter()
            .map(|(p, s)| json!({ "path": p, "jaccard_score": (s * 100.0).round() / 100.0 }))
            .collect();
        let payload = json!({
            "target_note": relations.target,
            "backlinks_count": backlinks.len(),
            "backlinks": backlinks,
            "outlinks_count": outlinks.len(),
            "outlinks": outlinks,
            "shared_tags_neighbors": neighbors_json,
        });
        to_text(&payload, "Failed to serialize: ")
    }

    /// Agrega una entrada a `~/.config/synapsers/feedback.md` con lo que le faltó al agente.
    fn execute_report_feedback(&self, args: &Value) -> McpCallToolResult {
        let text = |k: &str| {
            args.get(k)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
        };
        let (Some(title), Some(details)) = (text("title"), text("details")) else {
            return McpCallToolResult::error("Arguments 'title' and 'details' are required");
        };
        let path = crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl::resolve_config_dir()
            .join("feedback.md");
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // De qué conversación sale: agente, modelo y título (si la ventana lo sabe)
        let origin = self
            .conversation_hub
            .as_ref()
            .zip(self.caller.as_deref())
            .and_then(|(hub, caller)| hub.list(Some(caller)).ok())
            .and_then(|list| {
                list.as_array()?
                    .iter()
                    .find(|c| c["is_you"] == true)
                    .cloned()
            });
        let field = |k: &str| {
            origin
                .as_ref()
                .and_then(|o| o[k].as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let mut meta = format!(
            "- Date: {}\n- Date (unix): {secs}\n- Vault: {}\n",
            readable_date(secs),
            self.vault().display()
        );
        if let Some(agent) = field("agent") {
            meta.push_str(&format!("- Agent: {agent}\n"));
        }
        if let Some(model) = field("model") {
            meta.push_str(&format!("- Model: {model}\n"));
        }
        if let (Some(handle), title) = (field("handle"), field("title")) {
            meta.push_str(&format!(
                "- Conversation: {handle}{}\n",
                title.map(|t| format!(" · {t}")).unwrap_or_default()
            ));
        }
        if let Some(caller) = &self.caller {
            meta.push_str(&format!(
                "- History: .synapse/agent-sessions/{caller}.json\n"
            ));
        }
        let entry = format!("\n## {title}\n\n{meta}\n{details}\n");
        let result = (|| -> std::io::Result<()> {
            use std::io::Write;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let new = !path.exists();
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            if new {
                f.write_all(b"# Agent feedback on Synapse MCP\n")?;
            }
            f.write_all(entry.as_bytes())
        })();
        match result {
            Ok(()) => McpCallToolResult::text(format!("Feedback recorded at {}", path.display())),
            Err(e) => McpCallToolResult::error(format!("Could not save feedback: {e}")),
        }
    }

    fn execute_get_vault_git_status(&self) -> McpCallToolResult {
        let Some(ref git) = self.git_service else {
            return McpCallToolResult::error(
                "Git service is unavailable or this folder is not a repository.",
            );
        };
        match GetVaultVersionStateUseCase::new(&**git).execute(GetVaultVersionStateQuery {
            vault_path: self.vault(),
        }) {
            Ok(status) => to_text(&status, "Failed to serialize: "),
            Err(err) => McpCallToolResult::error(format!("Failed to query git: {}", err)),
        }
    }

    fn execute_read_pdf_document(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        let start_page = args.get("start_page").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let end_page = args
            .get("end_page")
            .and_then(|v| v.as_u64())
            .unwrap_or(start_page as u64 + 4) as usize;
        let query = ReadPdfDocumentQuery {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
            mode: args
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("outline_only")
                .to_string(),
            start_page,
            end_page: end_page.min(start_page + 4), // Límite estricto: máximo 5 páginas
            search_query: args
                .get("search_query")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
        let payload = match ReadPdfDocumentUseCase::new(
            &*self.pdf_document_repository,
            &*self.vault_item_repository,
        )
        .execute(query)
        {
            Err(err) => return McpCallToolResult::error(err),
            Ok(PdfReading::Pages(res)) => return to_text(&res, ""),
            Ok(PdfReading::Search { query, matches }) => json!({
                "relative_path": rel_path_str,
                "query": query,
                "matches_count": matches.len(),
                "matches": matches,
            }),
            Ok(PdfReading::Outline(outline)) => json!({
                "relative_path": rel_path_str,
                "total_pages": outline.total_pages,
                "file_size_bytes": outline.file_size_bytes,
            }),
        };
        to_text(&payload, "")
    }

    /// Modelo `.archimate` de la bóveda ya recorrido. Se guarda en la caché de análisis: recorrer
    /// el XML es lo costoso, y el agente suele hacer varias consultas seguidas al mismo modelo.
    fn archimate_model(
        &self,
        rel: &str,
    ) -> Result<Arc<crate::infrastructure::search::archimate_extractor::ArchimateModel>, McpCallToolResult> {
        use super::analysis_cache::{AnalysisCache, Analyzed};
        let rel_path = crate::domain::note::NoteRelativePath::new(rel).map_err(McpCallToolResult::error)?;
        let path = self.vault().join(rel_path.as_str());
        AnalysisCache::global().get_or_analyze(&path, "archimate", || {
            let bytes = std::fs::read(&path)
                .map_err(|e| McpCallToolResult::error(format!("Could not read '{rel}': {e}")))?;
            let model = crate::infrastructure::search::archimate_extractor::parse_archimate(&String::from_utf8_lossy(&bytes));
            Ok(Analyzed { bytes: model.approx_bytes(), value: model })
        })
    }

    fn execute_inspect_archimate(&self, args: &Value) -> McpCallToolResult {
        let Some(rel) = args.get("relative_path").and_then(|v| v.as_str()) else {
            return McpCallToolResult::error("Argument 'relative_path' is required");
        };
        let Some(query) = args.get("query").and_then(|v| v.as_str()).filter(|q| !q.trim().is_empty()) else {
            return McpCallToolResult::error("Argument 'query' is required");
        };
        if !rel.to_lowercase().ends_with(".archimate") {
            return McpCallToolResult::error("inspect_archimate_element only reads ArchiMate models (.archimate)");
        }
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5).clamp(1, 50) as usize;
        let model = match self.archimate_model(rel) {
            Ok(m) => m,
            Err(e) => return e,
        };
        let found = model.inspect(query, limit);
        to_text(
            &json!({
                "relative_path": rel,
                "query": query,
                "returned": found.len(),
                "elements": found,
            }),
            "Failed to serialize model: ",
        )
    }

    fn execute_extract_note_diagrams(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("Argument 'relative_path' is required"),
        };
        // draw.io (`.drawio`, o un PNG/SVG con el diagrama dentro): contenedores, figuras y conexiones
        let revision = args.get("revision").and_then(|v| v.as_str()).filter(|r| !r.trim().is_empty());
        let lower = rel_path_str.to_lowercase();
        if lower.ends_with(".drawio") || lower.ends_with(".png") || lower.ends_with(".svg") {
            return match self.drawio_source(rel_path_str, revision) {
                Err(e) => e,
                Ok(Some(xml)) => to_text(
                    &json!({
                        "relative_path": rel_path_str,
                        "revision": revision,
                        "diagram_type": "drawio",
                        "pages": crate::infrastructure::search::drawio_extractor::mxfile_structure(&xml),
                    }),
                    "Failed to serialize diagram: ",
                ),
                Ok(None) => McpCallToolResult::error(
                    "Image does not contain an embedded draw.io diagram: use render_diagram to view it.",
                ),
            };
        }
        let query = ExtractNoteDiagramsQuery {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
            diagram_type: args
                .get("diagram_type")
                .and_then(|v| v.as_str())
                .unwrap_or("all")
                .to_string(),
        };
        match ExtractNoteDiagramsUseCase::new(&self.note_repository, &*self.vault_item_repository)
            .execute(query)
        {
            Err(e) => McpCallToolResult::error(e),
            Ok(NoteDiagrams::DedicatedFile { kind, content }) => to_text(
                &json!({
                    "relative_path": rel_path_str,
                    "is_dedicated_file": true,
                    "diagrams": [{
                        "type": kind,
                        "start_line": 1,
                        "end_line": content.lines().count(),
                        "code": content,
                    }],
                }),
                "",
            ),
            Ok(NoteDiagrams::Blocks(blocks)) => {
                let diagrams: Vec<_> = blocks
                    .into_iter()
                    .map(|d| json!({ "type": d.kind, "start_line": d.start_line, "end_line": d.end_line, "code": d.code }))
                    .collect();
                to_text(
                    &json!({
                        "relative_path": rel_path_str,
                        "total_diagrams": diagrams.len(),
                        "diagrams": diagrams,
                    }),
                    "Failed to serialize diagrams: ",
                )
            }
        }
    }

    /// Retorna la lista de recursos MCP (notas accesibles como synapse://vault/...)
    pub fn list_resources(&self) -> McpResourcesListResult {
        let extensions = vec!["md".to_string(), "markdown".to_string()];
        let notes = ListNotesUseCase::new(&self.note_repository)
            .execute(ListNotesQuery {
                vault_path: self.vault(),
                extensions,
            })
            .unwrap_or_default();

        let resources = notes
            .into_iter()
            .map(|n| McpResource {
                uri: format!("synapse://vault/{}", n.relative_path.as_str()),
                name: n.title,
                description: Some(format!("Note at '{}'", n.relative_path.as_str())),
                mime_type: Some("text/markdown".to_string()),
            })
            .collect();

        McpResourcesListResult { resources }
    }

    /// Lee un recurso MCP por URI
    pub fn read_resource(&self, uri: &str) -> Result<McpReadResourceResult, String> {
        let prefix = "synapse://vault/";
        if !uri.starts_with(prefix) {
            return Err(format!(
                "Unsupported URI: {}. Must start with '{}'",
                uri, prefix
            ));
        }

        let rel_path_str = &uri[prefix.len()..];
        let note = ReadNoteUseCase::new(&self.note_repository).execute(ReadNoteQuery {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
        })?;

        Ok(McpReadResourceResult {
            contents: vec![McpResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/markdown".to_string()),
                text: note.content,
            }],
        })
    }

    /// Retorna la lista de prompts predefinidos
    pub fn list_prompts(&self) -> McpPromptsListResult {
        McpPromptsListResult {
            prompts: vec![
                McpPrompt {
                    name: "resumir_nota".to_string(),
                    description: "Generates a concise summary and extracts key concepts in OKF format.".to_string(),
                    arguments: vec![McpPromptArgument {
                        name: "relative_path".to_string(),
                        description: Some("Path of the note to summarize".to_string()),
                        required: true,
                    }],
                },
                McpPrompt {
                    name: "vincular_conceptos".to_string(),
                    description: "Analyzes a note's contents and suggests wikilinks and semantic relations to other notes.".to_string(),
                    arguments: vec![McpPromptArgument {
                        name: "relative_path".to_string(),
                        description: Some("Path of the note to link".to_string()),
                        required: true,
                    }],
                },
            ],
        }
    }

    /// Obtiene los mensajes correspondientes a un prompt predefinido
    pub fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<McpGetPromptResult, String> {
        let args = arguments.unwrap_or(Value::Object(serde_json::Map::new()));
        let rel_path_str = args
            .get("relative_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Argument 'relative_path' is required".to_string())?;

        let note = ReadNoteUseCase::new(&self.note_repository).execute(ReadNoteQuery {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
        })?;

        match name {
            "resumir_nota" => Ok(McpGetPromptResult {
                description: Some(format!("Summary of '{}'", note.title)),
                messages: vec![McpPromptMessage {
                    role: "user".to_string(),
                    content: McpPromptMessageContent {
                        content_type: "text".to_string(),
                        text: format!(
                            "Please summarize the following note and generate an OKF frontmatter block with tags and key concepts:\n\n---\nTitle: {}\nPath: {}\n---\n\n{}",
                            note.title,
                            note.relative_path.as_str(),
                            note.content
                        ),
                    },
                }],
            }),
            "vincular_conceptos" => Ok(McpGetPromptResult {
                description: Some(format!("Link concepts for '{}'", note.title)),
                messages: vec![McpPromptMessage {
                    role: "user".to_string(),
                    content: McpPromptMessageContent {
                        content_type: "text".to_string(),
                        text: format!(
                            "Review this note's content and suggest relevant semantic relations (relations.related) and [[...]] wikilinks:\n\n---\nTitle: {}\nPath: {}\n---\n\n{}",
                            note.title,
                            note.relative_path.as_str(),
                            note.content
                        ),
                    },
                }],
            }),
            other => Err(format!("Unknown prompt: '{}'", other)),
        }
    }
}

#[cfg(test)]
mod date_tests {
    use super::utc_date;

    #[test]
    fn fecha_utc_legible() {
        assert_eq!(utc_date(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(utc_date(1_794_989_944), "2026-11-18 08:19:04 UTC");
        // 29 de febrero de un año bisiesto
        assert_eq!(utc_date(951_782_400), "2000-02-29 00:00:00 UTC");
    }
}
