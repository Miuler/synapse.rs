use crate::application::dto::note::append_to_note_command::AppendToNoteCommand;
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

/// Respuesta de texto con el JSON formateado; si falla la serialización, error con el prefijo.
fn to_text<T: serde::Serialize + ?Sized>(payload: &T, error_prefix: &str) -> McpCallToolResult {
    match serde_json::to_string_pretty(payload) {
        Ok(text) => McpCallToolResult::text(text),
        Err(e) => McpCallToolResult::error(format!("{}{}", error_prefix, e)),
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
}

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
        }
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
        McpToolsListResult {
            tools: vec![
                // 1. search_notes
                McpTool {
                    name: "search_notes".to_string(),
                    description: "Busca notas por nombre, ruta o título dentro de la bóveda, con filtros opcionales por etiquetas (con soporte jerárquico sub-tags) y retorno de snippets.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "default": "",
                                "description": "Texto o palabra clave para filtrar notas (puede dejarse vacío si se usan filtros por tags)"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Filtro opcional de etiquetas OKF o sub-tags jerárquicos (ej. 'cachyos' coincide con 'linux/cachyos')"
                            },
                            "limit": {
                                "type": "number",
                                "description": "Número máximo de resultados a devolver (por defecto 20)"
                            }
                        }
                    }),
                },
                // 2. full_text_search
                McpTool {
                    name: "full_text_search".to_string(),
                    description: "Búsqueda profunda de texto completo (BM25) en el contenido de todos los documentos indexados, con filtros por carpeta, tags y telemetría de tokens.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Frase o palabras clave a buscar en el cuerpo de las notas"
                            },
                            "folder_path": {
                                "type": "string",
                                "description": "Subdirectorio para restringir el alcance de búsqueda"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Filtrar por etiquetas presentes en las notas"
                            },
                            "limit": {
                                "type": "number",
                                "description": "Número máximo de resultados (por defecto 20)"
                            }
                        },
                        "required": ["query"]
                    }),
                },
                // 3. search_in_note
                McpTool {
                    name: "search_in_note".to_string(),
                    description: "Busca texto o patrones dentro de una nota específica y devuelve las líneas coincidentes con su número y contexto circundante.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa de la nota donde buscar"
                            },
                            "query": {
                                "type": "string",
                                "description": "Texto o palabra a buscar dentro del archivo"
                            },
                            "context_lines": {
                                "type": "number",
                                "default": 2,
                                "description": "Número de líneas antes y después de cada coincidencia (máximo 10, por defecto 2)"
                            }
                        },
                        "required": ["relative_path", "query"]
                    }),
                },
                // 4. list_folder_contents
                McpTool {
                    name: "list_folder_contents".to_string(),
                    description: "Lista las notas y/o subdirectorios dentro de una carpeta de la bóveda, incluyendo estadísticas de líneas y tokens estimados.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "folder_path": {
                                "type": "string",
                                "description": "Ruta relativa de la carpeta a consultar (ej: 'proyectos/rust'). Dejar vacío para la raíz."
                            },
                            "include_notes": {
                                "type": "boolean",
                                "description": "Incluir las notas y archivos dentro de la carpeta (por defecto true)"
                            },
                            "include_folders": {
                                "type": "boolean",
                                "description": "Incluir los subdirectorios dentro de la carpeta (por defecto true)"
                            },
                            "recursive": {
                                "type": "boolean",
                                "description": "Si es true, lista recursivamente subcarpetas (por defecto false)"
                            },
                            "limit": {
                                "type": "number",
                                "default": 50,
                                "description": "Límite máximo de notas devueltas (por defecto 50, máx 200)"
                            },
                            "offset": {
                                "type": "number",
                                "default": 0,
                                "description": "Desplazamiento para paginación de notas (por defecto 0)"
                            }
                        }
                    }),
                },
                // 4.1. search_folders
                McpTool {
                    name: "search_folders".to_string(),
                    description: "Busca carpetas por coincidencia difusa de nombre o ruta, devolviendo huella semántica (conteo de notas, tags dominantes y notas de muestra) para desambiguar carpetas homónimas.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Término o nombre parcial de la carpeta a buscar (ej: 'azure', 'facturas', 'cli')"
                            },
                            "limit": {
                                "type": "number",
                                "default": 20,
                                "description": "Límite de carpetas coincidentes a devolver (por defecto 20)"
                            },
                            "offset": {
                                "type": "number",
                                "default": 0,
                                "description": "Desplazamiento para paginación (por defecto 0)"
                            }
                        }
                    }),
                },
                // 4.2. move_folder_contents
                McpTool {
                    name: "move_folder_contents".to_string(),
                    description: "Mueve todas las notas de una carpeta de origen a una carpeta de destino, con soporte para estrategia de colisiones, extensión y simulacro (dry_run).".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "source_folder": {
                                "type": "string",
                                "description": "Ruta relativa de la carpeta origen (ej: 'descargas' o 'proyectos/azure')"
                            },
                            "target_folder": {
                                "type": "string",
                                "description": "Ruta relativa de la carpeta destino (ej: 'archivo/azure' o 'clientes/bcp')"
                            },
                            "extension": {
                                "type": "string",
                                "description": "Filtro opcional por extensión de archivo a mover (ej: 'md' o 'pdf')"
                            },
                            "conflict_strategy": {
                                "type": "string",
                                "enum": ["skip", "rename", "overwrite"],
                                "default": "skip",
                                "description": "Estrategia ante archivos duplicados en destino: 'skip' (omite), 'rename' (añade sufijo numérico) o 'overwrite' (sobrescribe)"
                            },
                            "dry_run": {
                                "type": "boolean",
                                "default": false,
                                "description": "Si es true, solo simula y devuelve la lista de archivos que se moverían sin alterar el disco"
                            }
                        },
                        "required": ["source_folder", "target_folder"]
                    }),
                },
                // 5. list_sibling_notes
                McpTool {
                    name: "list_sibling_notes".to_string(),
                    description: "A partir de una nota de referencia, lista todas las demás notas ubicadas en la misma carpeta.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa de la nota de referencia (ej: 'notas/arquitectura.md')"
                            },
                            "include_self": {
                                "type": "boolean",
                                "description": "Incluir también la nota de referencia en el resultado (por defecto false)"
                            },
                            "include_folders": {
                                "type": "boolean",
                                "description": "Incluir subcarpetas hermanas en el mismo directorio (por defecto false)"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 6. read_note
                McpTool {
                    name: "read_note".to_string(),
                    description: "Lee una nota Markdown y sus metadatos OKF. Soporta modos 'full', 'chunk', 'section' y 'metadata_only'/'outline_only', proveyendo content_hash (OCC), TOC y detección de bloques.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa del archivo dentro de la bóveda (ej: 'notas/arquitectura.md')"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["full", "chunk", "section", "metadata_only", "outline_only"],
                                "default": "full",
                                "description": "'full': lee toda la nota. 'chunk': lee un rango acotado de líneas. 'section': extrae solo un encabezado. 'metadata_only'/'outline_only': metadatos OKF, TOC y estadísticas sin el cuerpo."
                            },
                            "outline_only": {
                                "type": "boolean",
                                "description": "Si es true, devuelve únicamente el TOC (encabezados con números de línea) y metadatos sin el cuerpo completo"
                            },
                            "section": {
                                "type": "string",
                                "description": "Nombre del encabezado H1/H2 a extraer exclusivamente (alias directo de section_heading)"
                            },
                            "start_line": {
                                "type": "number",
                                "description": "Línea inicial (base 1) cuando mode='chunk' (por defecto 1)"
                            },
                            "line_count": {
                                "type": "number",
                                "description": "Cantidad de líneas a leer en mode='chunk' (por defecto 150, máximo 500)"
                            },
                            "section_heading": {
                                "type": "string",
                                "description": "Texto del encabezado a extraer cuando mode='section' (ej: '## Canal ACP')"
                            },
                            "include_line_numbers": {
                                "type": "boolean",
                                "default": false,
                                "description": "Prefija cada línea devuelta con su número (ej: '42: texto') para facilitar patch_note."
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 7. write_note
                McpTool {
                    name: "write_note".to_string(),
                    description: "Crea una nota o reemplaza una existente, con protección de hash OCC, freno anti-truncamiento y modo dry_run. Reemplazar exige expected_hash (recomendado) u overwrite: true; si no, falla. La respuesta indica en 'created' si la nota es nueva y en 'warning' si su carpeta no existía.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, true)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa del archivo (ej: 'proyectos/nuevo.md')"
                            },
                            "title": {
                                "type": "string",
                                "description": "Título principal del documento (opcional si la nota ya existe)"
                            },
                            "content": {
                                "type": "string",
                                "description": "Cuerpo en formato Markdown"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Lista de etiquetas de taxonomía"
                            },
                            "aliases": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Nombres alternativos para la nota"
                            },
                            "expected_hash": {
                                "type": "string",
                                "description": "Hash obtenido previamente en read_note para control de concurrencia optimista"
                            },
                            "overwrite": {
                                "type": "boolean",
                                "default": false,
                                "description": "Permite reemplazar una nota existente sin haberla leído. Es preferible leerla y enviar expected_hash, que basta por sí solo. Sin ninguno de los dos, escribir sobre una nota existente falla"
                            },
                            "allow_large_overwrite": {
                                "type": "boolean",
                                "default": false,
                                "description": "Permite sobrescritura incluso si reduce más del 40% del documento original (>150 líneas)"
                            },
                            "dry_run": {
                                "type": "boolean",
                                "default": false,
                                "description": "Si es true, valida y retorna el unified_diff sin modificar disco"
                            }
                        },
                        "required": ["relative_path", "content"]
                    }),
                },
                // 8. patch_note
                McpTool {
                    name: "patch_note".to_string(),
                    description: "Modifica un fragmento quirúrgico de una nota reemplazando un bloque de texto exacto con tolerancia de anclaje difuso y control OCC.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa de la nota a editar"
                            },
                            "old_text": {
                                "type": "string",
                                "description": "Texto original exacto a reemplazar"
                            },
                            "new_text": {
                                "type": "string",
                                "description": "Nuevo texto en Markdown que reemplazará a old_text"
                            },
                            "start_line": {
                                "type": "number",
                                "description": "Línea inicial estimada del bloque (base 1)"
                            },
                            "end_line": {
                                "type": "number",
                                "description": "Línea final estimada del bloque"
                            },
                            "expected_hash": {
                                "type": "string",
                                "description": "Hash previo para verificar que la nota no cambió concurrentemente"
                            },
                            "dry_run": {
                                "type": "boolean",
                                "default": false,
                                "description": "Si es true, calcula el unified_diff sin alterar disco"
                            }
                        },
                        "required": ["relative_path", "old_text", "new_text"]
                    }),
                },
                // 9. append_to_note
                McpTool {
                    name: "append_to_note".to_string(),
                    description: "Agrega contenido al final de una nota o dentro de una sección específica de encabezado Markdown.".to_string(),
                    annotations: Some(McpToolAnnotations::write(false, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa de la nota"
                            },
                            "content": {
                                "type": "string",
                                "description": "Texto Markdown a insertar"
                            },
                            "section_heading": {
                                "type": "string",
                                "description": "Encabezado opcional donde insertar (ej: '## Tareas'). Si se omite, se agrega al final del archivo."
                            },
                            "position": {
                                "type": "string",
                                "enum": ["start_of_section", "end_of_section"],
                                "default": "end_of_section",
                                "description": "Posición dentro de la sección elegida"
                            }
                        },
                        "required": ["relative_path", "content"]
                    }),
                },
                // 10. update_note_metadata
                McpTool {
                    name: "update_note_metadata".to_string(),
                    description: "Actualiza atómicamente los metadatos OKF (título, tags, aliases, custom_properties) del frontmatter YAML sin tocar el cuerpo de la nota.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, true)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa de la nota"
                            },
                            "title": {
                                "type": "string",
                                "description": "Nuevo título opcional"
                            },
                            "add_tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Etiquetas a añadir (sin duplicados)"
                            },
                            "remove_tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Etiquetas a eliminar"
                            },
                            "aliases": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Lista de aliases a establecer"
                            },
                            "custom_properties": {
                                "type": "object",
                                "description": "Pares clave-valor adicionales para el frontmatter"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 11. move_notes
                McpTool {
                    name: "move_notes".to_string(),
                    description: "Mueve notas a una carpeta de destino con soporte de reescritura automática de wikilinks en la bóveda.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "notes": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Lista de rutas de notas a mover"
                            },
                            "target_folder": {
                                "type": "string",
                                "description": "Carpeta destino relativa"
                            },
                            "create_target_folder": {
                                "type": "boolean",
                                "default": true,
                                "description": "Crear carpeta si no existe"
                            },
                            "overwrite": {
                                "type": "boolean",
                                "default": false,
                                "description": "Sobrescribir si ya existe en destino"
                            },
                            "update_wikilinks": {
                                "type": "boolean",
                                "default": false,
                                "description": "Reescribir enlaces entrantes en otras notas para apuntar a la nueva ubicación"
                            }
                        },
                        "required": ["notes", "target_folder"]
                    }),
                },
                // 12. delete_note
                McpTool {
                    name: "delete_note".to_string(),
                    description: "Elimina permanentemente una nota del sistema de archivos dentro de la bóveda. Soporta control de concurrencia optimista mediante expected_hash.".to_string(),
                    annotations: Some(McpToolAnnotations::write(true, false)),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa de la nota a eliminar dentro de la bóveda (ej. 'carpeta/nota.md')"
                            },
                            "expected_hash": {
                                "type": "string",
                                "description": "Hash SHA-256 opcional (obtenido en read_note). Si se proporciona, previene la eliminación si el contenido cambió"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 13. get_backlinks_and_relations
                McpTool {
                    name: "get_backlinks_and_relations".to_string(),
                    description: "Obtiene backlinks con snippets contextuales, outlinks no resueltos (huérfanos) y top 5 vecinos por similitud Jaccard de tags.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa de la nota a analizar"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 14. get_vault_git_status
                McpTool {
                    name: "get_vault_git_status".to_string(),
                    description: "Obtiene el estado de control de versiones Git de la bóveda (rama, index, worktree, stash).".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {}
                    }),
                },
                // 15. read_pdf_document
                McpTool {
                    name: "read_pdf_document".to_string(),
                    description: "Inspecciona documentos PDF: índice/metadatos, lectura por rango de páginas (máximo 5) o búsqueda interna.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa del archivo PDF (ej: 'docs/paper.pdf')"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["outline_only", "pages", "search"],
                                "default": "outline_only",
                                "description": "'outline_only': total de páginas y metadatos. 'pages': extrae texto entre start_page y end_page. 'search': busca un término en el PDF."
                            },
                            "start_page": {
                                "type": "number",
                                "description": "Página inicial (base 1, por defecto 1)"
                            },
                            "end_page": {
                                "type": "number",
                                "description": "Página final (máximo 5 páginas por llamada)"
                            },
                            "search_query": {
                                "type": "string",
                                "description": "Término a buscar cuando mode='search'"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 16. extract_note_diagrams
                McpTool {
                    name: "extract_note_diagrams".to_string(),
                    description: "Extrae todos los bloques de código de diagramas (Mermaid, D2lang) de una nota con números de línea o lee archivos .d2 / .mmd.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa de la nota Markdown o archivo .d2 / .mmd"
                            },
                            "diagram_type": {
                                "type": "string",
                                "enum": ["all", "mermaid", "d2"],
                                "default": "all",
                                "description": "Filtrar por tipo de diagrama"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                // 17. list_vault_tree (alias retrocompatible)
                McpTool {
                    name: "list_vault_tree".to_string(),
                    description: "Alias retrocompatible de list_folder_contents.".to_string(),
                    annotations: Some(McpToolAnnotations::read_only()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "parent_path": {
                                "type": "string",
                                "description": "Carpeta relativa"
                            }
                        }
                    }),
                },
            ],
        }
    }

    /// Ejecuta una herramienta MCP por nombre
    pub fn call_tool(&self, name: &str, arguments: Option<Value>) -> McpCallToolResult {
        let args = arguments.unwrap_or(Value::Object(serde_json::Map::new()));
        match name {
            "search_notes" => self.execute_search_notes(&args),
            "search_folders" => self.execute_search_folders(&args),
            "move_folder_contents" => self.execute_move_folder_contents_tool(&args),
            "full_text_search" => self.execute_full_text_search(&args),
            "search_in_note" => self.execute_search_in_note(&args),
            "list_folder_contents" => self.execute_list_folder_contents_tool(&args),
            "list_vault_tree" => {
                let parent = args
                    .get("parent_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.execute_list_folder_contents(parent, true, true, false, None, None, None)
            }
            "list_sibling_notes" => self.execute_list_sibling_notes(&args),
            "read_note" => self.execute_read_note(&args),
            "write_note" => self.execute_write_note(&args),
            "patch_note" => self.execute_patch_note(&args),
            "append_to_note" => self.execute_append_to_note(&args),
            "update_note_metadata" => self.execute_update_note_metadata(&args),
            "move_notes" => self.execute_move_notes_tool(&args),
            "delete_note" => self.execute_delete_note(&args),
            "get_backlinks_and_relations" => self.execute_get_backlinks_and_relations(&args),
            "get_vault_git_status" => self.execute_get_vault_git_status(),
            "read_pdf_document" => self.execute_read_pdf_document(&args),
            "extract_note_diagrams" => self.execute_extract_note_diagrams(&args),
            other => McpCallToolResult::error(format!("Herramienta desconocida: '{}'", other)),
        }
    }

    // ==========================================
    // Herramientas: traducen JSON ↔ Command/Query y delegan en casos de uso
    // ==========================================

    fn vault(&self) -> PathBuf {
        self.vault_path.clone()
    }

    fn execute_search_notes(&self, args: &Value) -> McpCallToolResult {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .map(|q| q.trim().to_lowercase())
            .unwrap_or_default();
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let tags = string_list(args.get("tags"));

        match SearchNotesUseCase::new(&self.note_repository).execute(SearchNotesQuery {
            vault_path: self.vault(),
            query,
            limit,
            tags,
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
                to_text(&filtered, "Error al serializar notas: ")
            }
            Err(err) => McpCallToolResult::error(err),
        }
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
        to_text(&payload, "Error serializando carpetas: ")
    }

    fn execute_move_folder_contents_tool(&self, args: &Value) -> McpCallToolResult {
        let source_folder = match args.get("source_folder").and_then(|v| v.as_str()) {
            Some(sf) => sf.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("El argumento 'source_folder' es obligatorio"),
        };
        let target_folder = match args.get("target_folder").and_then(|v| v.as_str()) {
            Some(tf) => tf.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("El argumento 'target_folder' es obligatorio"),
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
            None => return McpCallToolResult::error("El argumento 'query' es obligatorio"),
        };
        if query_str.is_empty() {
            return McpCallToolResult::text("[]");
        }
        let Some(ref fts) = self.full_text_index else {
            return McpCallToolResult::error(
                "El índice Tantivy de búsqueda de texto completo no está disponible.",
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
        };
        match SearchNotesFullTextUseCase::new(&**fts, &self.note_repository).execute(query) {
            Ok(result) => {
                let hits: Vec<_> = result
                    .hits
                    .into_iter()
                    .map(|h| {
                        json!({
                            "path": h.hit.note_path,
                            "title": h.hit.title,
                            "score": h.hit.score,
                            "heading": h.hit.heading,
                            "snippet": h.hit.snippet,
                            "total_lines": h.total_lines,
                            "estimated_tokens": h.estimated_tokens,
                            "matched_terms": h.hit.matched_terms,
                        })
                    })
                    .collect();
                let results = json!({
                    "total_matches": result.total,
                    "returned_matches": hits.len(),
                    "hits": hits,
                });
                to_text(&results, "Error al serializar: ")
            }
            Err(err) => McpCallToolResult::error(err),
        }
    }

    fn execute_search_in_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return McpCallToolResult::error("El argumento 'query' es obligatorio"),
        };
        let context_lines = args
            .get("context_lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        let matches =
            match SearchInNoteUseCase::new(&self.note_repository).execute(SearchInNoteQuery {
                vault_path: self.vault(),
                relative_path: rel_path_str.to_string(),
                query: query.to_string(),
                context_lines,
            }) {
                Ok(m) => m,
                Err(e) => return McpCallToolResult::error(e),
            };
        let matches: Vec<_> = matches
            .into_iter()
            .map(|m| json!({ "line_number": m.line_number, "matched_line": m.matched_line, "context": m.context }))
            .collect();
        let payload = json!({
            "relative_path": rel_path_str,
            "total_matches": matches.len(),
            "matches": matches,
        });
        to_text(&payload, "Error al serializar: ")
    }

    fn execute_read_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
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
                            "El argumento 'section' o 'section_heading' es requerido en mode='section'",
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
                            "Sección '{}' no encontrada en la nota. Secciones disponibles: {:?}",
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
        to_text(&payload, "")
    }

    fn execute_write_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return McpCallToolResult::error("El argumento 'content' es obligatorio"),
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
                    "Nota '{}' {} exitosamente.",
                    rel_path_str,
                    if created { "creada" } else { "sobrescrita" }
                ),
            }),
        };
        if let Some(folder) = missing_folder {
            payload["warning"] = json!(format!(
                "La carpeta '{}' no existía en la bóveda. Comprueba que el nombre es correcto.",
                folder
            ));
        }
        to_text(&payload, "")
    }

    fn execute_patch_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        let old_text = match args.get("old_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return McpCallToolResult::error("El argumento 'old_text' es obligatorio"),
        };
        let new_text = match args.get("new_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return McpCallToolResult::error("El argumento 'new_text' es obligatorio"),
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
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        let content_to_append = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return McpCallToolResult::error("El argumento 'content' es obligatorio"),
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
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
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
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        if rel_path_str.is_empty() {
            return McpCallToolResult::error("El argumento 'relative_path' no puede estar vacío");
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
        to_text(&payload, "Error al serializar contenidos: ")
    }

    fn execute_move_notes_tool(&self, args: &Value) -> McpCallToolResult {
        let notes_val = match args.get("notes").and_then(|v| v.as_array()) {
            Some(n) => n,
            None => {
                return McpCallToolResult::error(
                    "El argumento 'notes' debe ser una lista de rutas",
                );
            }
        };
        let target_folder = match args.get("target_folder").and_then(|v| v.as_str()) {
            Some(tf) => tf,
            None => return McpCallToolResult::error("El argumento 'target_folder' es obligatorio"),
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
        to_text(&payload, "Error al serializar move_notes: ")
    }

    fn execute_delete_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        if rel_path_str.is_empty() {
            return McpCallToolResult::error("El argumento 'relative_path' no puede estar vacío");
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
                    "message": "Nota eliminada correctamente"
                }),
                "",
            ),
            Err(e) => McpCallToolResult::error(e),
        }
    }

    fn execute_get_backlinks_and_relations(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
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
            .map(|o| json!({ "target": o.target, "alias": o.alias, "is_embed": o.is_embed, "is_resolved": o.is_resolved }))
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
        to_text(&payload, "Error al serializar: ")
    }

    fn execute_get_vault_git_status(&self) -> McpCallToolResult {
        let Some(ref git) = self.git_service else {
            return McpCallToolResult::error(
                "El servicio Git no está disponible o la carpeta no es un repositorio.",
            );
        };
        match GetVaultVersionStateUseCase::new(&**git).execute(GetVaultVersionStateQuery {
            vault_path: self.vault(),
        }) {
            Ok(status) => to_text(&status, "Error al serializar: "),
            Err(err) => McpCallToolResult::error(format!("Error al consultar git: {}", err)),
        }
    }

    fn execute_read_pdf_document(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
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

    fn execute_extract_note_diagrams(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
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
                    "Error al serializar diagramas: ",
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
                description: Some(format!("Nota en '{}'", n.relative_path.as_str())),
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
                "URI no soportada: {}. Debe empezar con '{}'",
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
                    description: "Genera un resumen conciso y extrae conceptos clave en formato OKF.".to_string(),
                    arguments: vec![McpPromptArgument {
                        name: "relative_path".to_string(),
                        description: Some("Ruta de la nota a resumir".to_string()),
                        required: true,
                    }],
                },
                McpPrompt {
                    name: "vincular_conceptos".to_string(),
                    description: "Analiza el contenido de una nota y sugiere wikilinks y relaciones semánticas con otras notas.".to_string(),
                    arguments: vec![McpPromptArgument {
                        name: "relative_path".to_string(),
                        description: Some("Ruta de la nota a vincular".to_string()),
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
            .ok_or_else(|| "El argumento 'relative_path' es requerido".to_string())?;

        let note = ReadNoteUseCase::new(&self.note_repository).execute(ReadNoteQuery {
            vault_path: self.vault(),
            relative_path: rel_path_str.to_string(),
        })?;

        match name {
            "resumir_nota" => Ok(McpGetPromptResult {
                description: Some(format!("Resumen de '{}'", note.title)),
                messages: vec![McpPromptMessage {
                    role: "user".to_string(),
                    content: McpPromptMessageContent {
                        content_type: "text".to_string(),
                        text: format!(
                            "Por favor resume la siguiente nota y genera un bloque frontmatter OKF con tags y conceptos clave:\n\n---\nTitulo: {}\nRuta: {}\n---\n\n{}",
                            note.title,
                            note.relative_path.as_str(),
                            note.content
                        ),
                    },
                }],
            }),
            "vincular_conceptos" => Ok(McpGetPromptResult {
                description: Some(format!("Vincular conceptos para '{}'", note.title)),
                messages: vec![McpPromptMessage {
                    role: "user".to_string(),
                    content: McpPromptMessageContent {
                        content_type: "text".to_string(),
                        text: format!(
                            "Revisa el contenido de esta nota y sugiere posibles relaciones semánticas (relations.related) y wikilinks [[...]] pertinentes:\n\n---\nTitulo: {}\nRuta: {}\n---\n\n{}",
                            note.title,
                            note.relative_path.as_str(),
                            note.content
                        ),
                    },
                }],
            }),
            other => Err(format!("Prompt desconocido: '{}'", other)),
        }
    }
}
