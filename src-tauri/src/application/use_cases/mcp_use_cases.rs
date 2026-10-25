use crate::domain::models::frontmatter::NoteFrontmatter;
use crate::domain::models::mcp::*;
use crate::domain::models::note::Note;
use crate::domain::repositories::note_repository::NoteRepository;
use crate::domain::services::frontmatter_service;
use crate::domain::value_objects::note_path::NoteRelativePath;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Casos de uso de MCP para consultar y manipular la bóveda desde agentes de IA.
pub struct McpUseCases<R: NoteRepository> {
    vault_path: PathBuf,
    note_repository: R,
}

impl<R: NoteRepository> McpUseCases<R> {
    pub fn new(vault_path: PathBuf, note_repository: R) -> Self {
        Self {
            vault_path,
            note_repository,
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
                tools: Some(McpToolsCapability { list_changed: false }),
                resources: Some(McpResourcesCapability {
                    subscribe: false,
                    list_changed: false,
                }),
                prompts: Some(McpPromptsCapability { list_changed: false }),
            },
            server_info: McpServerInfo {
                name: "synapse-mcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    /// Retorna la lista de herramientas disponibles
    pub fn list_tools(&self) -> McpToolsListResult {
        McpToolsListResult {
            tools: vec![
                McpTool {
                    name: "search_notes".to_string(),
                    description: "Busca notas por nombre, ruta o título dentro de la bóveda de Synapse.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Texto o palabra clave para filtrar notas"
                            },
                            "limit": {
                                "type": "number",
                                "description": "Número máximo de resultados a devolver (por defecto 20)"
                            }
                        },
                        "required": ["query"]
                    }),
                },
                McpTool {
                    name: "read_note".to_string(),
                    description: "Lee el contenido completo y los metadatos OKF (YAML frontmatter) de una nota.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa del archivo dentro de la bóveda (ej: 'notas/arquitectura.md')"
                            }
                        },
                        "required": ["relative_path"]
                    }),
                },
                McpTool {
                    name: "write_note".to_string(),
                    description: "Crea o actualiza una nota en Markdown preservando o actualizando sus metadatos OKF.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "relative_path": {
                                "type": "string",
                                "description": "Ruta relativa del archivo (ej: 'proyectos/nuevo.md')"
                            },
                            "title": {
                                "type": "string",
                                "description": "Título principal del documento"
                            },
                            "content": {
                                "type": "string",
                                "description": "Cuerpo en formato Markdown (sin incluir los delimitadores --- de frontmatter si se proporcionan metadatos separados)"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Lista opcional de etiquetas de taxonomía"
                            },
                            "aliases": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Nombres o títulos alternativos para la nota"
                            }
                        },
                        "required": ["relative_path", "title", "content"]
                    }),
                },
                McpTool {
                    name: "list_vault_tree".to_string(),
                    description: "Lista los archivos y directorios inmediatos en la bóveda o en una subcarpeta.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "parent_path": {
                                "type": "string",
                                "description": "Carpeta relativa a inspeccionar. Deja vacío para la raíz de la bóveda."
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
            "search_notes" => {
                let query = match args.get("query").and_then(|v| v.as_str()) {
                    Some(q) => q.trim().to_lowercase(),
                    None => return McpCallToolResult::error("El argumento 'query' es obligatorio"),
                };
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

                let extensions = vec!["md".to_string(), "markdown".to_string()];
                match self.note_repository.list_notes(&self.vault_path, &extensions) {
                    Ok(notes) => {
                        let filtered: Vec<_> = notes
                            .into_iter()
                            .filter(|n| {
                                n.title.to_lowercase().contains(&query)
                                    || n.relative_path.as_str().to_lowercase().contains(&query)
                                    || n.frontmatter.as_ref().map_or(false, |fm| {
                                        fm.tags.iter().any(|t| t.to_lowercase().contains(&query))
                                            || fm.aliases.iter().any(|a| a.to_lowercase().contains(&query))
                                    })
                            })
                            .take(limit)
                            .map(|n| {
                                json!({
                                    "title": n.title,
                                    "relative_path": n.relative_path.as_str(),
                                    "tags": n.frontmatter.as_ref().map(|f| &f.tags),
                                    "aliases": n.frontmatter.as_ref().map(|f| &f.aliases),
                                })
                            })
                            .collect();

                        match serde_json::to_string_pretty(&filtered) {
                            Ok(text) => McpCallToolResult::text(text),
                            Err(e) => McpCallToolResult::error(format!("Error al serializar notas: {}", e)),
                        }
                    }
                    Err(err) => McpCallToolResult::error(format!("Error al buscar notas: {}", err)),
                }
            }
            "read_note" => {
                let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
                };

                let relative_path = match NoteRelativePath::new(rel_path_str) {
                    Ok(p) => p,
                    Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
                };

                match self.note_repository.read_note(&self.vault_path, &relative_path) {
                    Ok(note) => {
                        let payload = json!({
                            "title": note.title,
                            "relative_path": note.relative_path.as_str(),
                            "frontmatter": note.frontmatter,
                            "content": note.content,
                        });
                        match serde_json::to_string_pretty(&payload) {
                            Ok(text) => McpCallToolResult::text(text),
                            Err(e) => McpCallToolResult::error(format!("Error al serializar: {}", e)),
                        }
                    }
                    Err(err) => McpCallToolResult::error(format!("Error al leer nota: {}", err)),
                }
            }
            "write_note" => {
                let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
                };
                let title = match args.get("title").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return McpCallToolResult::error("El argumento 'title' es obligatorio"),
                };
                let content = match args.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return McpCallToolResult::error("El argumento 'content' es obligatorio"),
                };

                let relative_path = match NoteRelativePath::new(rel_path_str) {
                    Ok(p) => p,
                    Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
                };

                // Si el contenido ya incluye delimitadores `---`, usar tal cual;
                // si no, estructurar el frontmatter OKF si se especificaron tags o título.
                let final_body = if content.trim_start().starts_with("---") {
                    content.to_string()
                } else {
                    let mut fm = NoteFrontmatter::default();
                    fm.title = Some(title.to_string());
                    if let Some(tags_val) = args.get("tags").and_then(|v| v.as_array()) {
                        fm.tags = tags_val
                            .iter()
                            .filter_map(|t| t.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                    if let Some(aliases_val) = args.get("aliases").and_then(|v| v.as_array()) {
                        fm.aliases = aliases_val
                            .iter()
                            .filter_map(|a| a.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                    frontmatter_service::serialize_frontmatter_and_content(Some(&fm), content)
                };

                let note = Note::with_encoding(
                    relative_path,
                    "".to_string(),
                    title.to_string(),
                    final_body,
                    "UTF-8".to_string(),
                );

                match self.note_repository.save_note(&self.vault_path, &note) {
                    Ok(_) => McpCallToolResult::text(format!("Nota '{}' guardada exitosamente.", rel_path_str)),
                    Err(err) => McpCallToolResult::error(format!("Error al guardar la nota: {}", err)),
                }
            }
            "list_vault_tree" => {
                let parent = args.get("parent_path").and_then(|v| v.as_str()).unwrap_or("");
                let clean_parent = parent.trim_matches('/').replace('\\', "/");
                let target_dir = if clean_parent.is_empty() {
                    self.vault_path.clone()
                } else {
                    self.vault_path.join(&clean_parent)
                };

                if !target_dir.exists() || !target_dir.is_dir() {
                    return McpCallToolResult::error(format!("Directorio no encontrado: {}", clean_parent));
                }

                let mut entries_list = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&target_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') || name == "target" || name == "node_modules" {
                            continue;
                        }
                        let is_folder = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        let rel = if clean_parent.is_empty() {
                            name.clone()
                        } else {
                            format!("{}/{}", clean_parent, name)
                        };
                        entries_list.push(json!({
                            "name": name,
                            "relative_path": rel,
                            "is_folder": is_folder,
                        }));
                    }
                }

                match serde_json::to_string_pretty(&entries_list) {
                    Ok(text) => McpCallToolResult::text(text),
                    Err(e) => McpCallToolResult::error(format!("Error al serializar árbol: {}", e)),
                }
            }
            other => McpCallToolResult::error(format!("Herramienta desconocida: '{}'", other)),
        }
    }

    /// Retorna la lista de recursos MCP (notas accesibles como synapse://vault/...)
    pub fn list_resources(&self) -> McpResourcesListResult {
        let extensions = vec!["md".to_string(), "markdown".to_string()];
        let notes = self
            .note_repository
            .list_notes(&self.vault_path, &extensions)
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
            return Err(format!("URI no soportada: {}. Debe empezar con '{}'", uri, prefix));
        }

        let rel_path_str = &uri[prefix.len()..];
        let relative_path = NoteRelativePath::new(rel_path_str)?;
        let note = self.note_repository.read_note(&self.vault_path, &relative_path)?;

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
    pub fn get_prompt(&self, name: &str, arguments: Option<Value>) -> Result<McpGetPromptResult, String> {
        let args = arguments.unwrap_or(Value::Object(serde_json::Map::new()));
        let rel_path_str = args
            .get("relative_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "El argumento 'relative_path' es requerido".to_string())?;

        let relative_path = NoteRelativePath::new(rel_path_str)?;
        let note = self.note_repository.read_note(&self.vault_path, &relative_path)?;

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
