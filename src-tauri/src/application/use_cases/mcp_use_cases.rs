use crate::domain::models::full_text::FullTextQuery;
use crate::domain::models::mcp::*;
use crate::domain::models::note::Note;
use crate::domain::repositories::note_repository::NoteRepository;
use crate::domain::services::frontmatter_service;
use crate::domain::services::full_text_index::FullTextIndex;
use crate::domain::services::git_service::GitPort;
use crate::domain::services::link_resolution::{extract_wikilinks, normalize_link_target};
use crate::domain::services::tag_service::{self, matches_tag};
use crate::domain::value_objects::note_path::NoteRelativePath;
use crate::infrastructure::search::pdf_extractor;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use similar::{ChangeTag, TextDiff};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Extrae un fragmento de texto limpio (sin frontmatter YAML, sin encabezados Markdown vacíos) para resumen o snippet.
pub fn extract_clean_snippet(content: &str, max_chars: usize) -> String {
    let (_, body) = frontmatter_service::parse_frontmatter_and_content(content);
    let mut snippet_parts = Vec::new();
    let mut char_count = 0;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("```") || trimmed == "---" {
            continue;
        }
        // Limpiar '#' de encabezados
        let clean_line = trimmed.trim_start_matches('#').trim();
        if clean_line.is_empty() {
            continue;
        }

        snippet_parts.push(clean_line);
        char_count += clean_line.len() + 1;
        if char_count >= max_chars {
            break;
        }
    }

    let joined = snippet_parts.join(" ");
    if joined.chars().count() > max_chars {
        let truncated: String = joined.chars().take(max_chars).collect();
        format!("{}...", truncated.trim_end())
    } else {
        joined
    }
}

/// Aplica reemplazo quirúrgico de texto con tolerancia progresiva:
/// 1. Coincidencia exacta (usando start_line para desambiguar si hay múltiples ocurrencias).
/// 2. Normalización de saltos de línea (\r\n vs \n).
/// 3. Tolerancia de espacios en blanco al final de línea (.trim_end()).
/// 4. Tolerancia de indentación (.trim()).
pub fn apply_surgical_patch(
    full_content: &str,
    old_text: &str,
    new_text: &str,
    start_line_hint: Option<usize>,
) -> Result<String, String> {
    if old_text.is_empty() {
        return Err("El argumento 'old_text' no puede estar vacío".to_string());
    }

    // 1. Nivel 1: Coincidencia exacta
    let occurrences: Vec<_> = full_content.match_indices(old_text).collect();
    if occurrences.len() == 1 {
        let (idx, _) = occurrences[0];
        return Ok(format!("{}{}{}", &full_content[..idx], new_text, &full_content[idx + old_text.len()..]));
    } else if occurrences.len() > 1 {
        // Desambiguar usando start_line si fue provisto
        if let Some(target_line) = start_line_hint {
            let mut best_match = None;
            let mut min_diff = usize::MAX;

            for (byte_idx, _) in &occurrences {
                let line_num = full_content[..*byte_idx].lines().count() + 1;
                let diff = (line_num as isize - target_line as isize).unsigned_abs();
                if diff < min_diff {
                    min_diff = diff;
                    best_match = Some(*byte_idx);
                }
            }

            if let Some(idx) = best_match {
                return Ok(format!("{}{}{}", &full_content[..idx], new_text, &full_content[idx + old_text.len()..]));
            }
        }
        return Err(format!(
            "Ambigüedad: 'old_text' aparece {} veces en el archivo. Proporciona 'start_line' o un fragmento más extenso para unicidad.",
            occurrences.len()
        ));
    }

    // 2. Nivel 2: Normalización CRLF a LF
    let norm_full = full_content.replace("\r\n", "\n");
    let norm_old = old_text.replace("\r\n", "\n");
    let norm_new = new_text.replace("\r\n", "\n");
    let norm_occurrences: Vec<_> = norm_full.match_indices(&norm_old).collect();
    if norm_occurrences.len() == 1 {
        let (idx, _) = norm_occurrences[0];
        let patched = format!("{}{}{}", &norm_full[..idx], norm_new, &norm_full[idx + norm_old.len()..]);
        // Si el archivo original usaba predominantemente CRLF, restaurar \r\n
        return Ok(if full_content.contains("\r\n") { patched.replace('\n', "\r\n") } else { patched });
    } else if norm_occurrences.len() > 1 {
        if let Some(target_line) = start_line_hint {
            let mut best_match = None;
            let mut min_diff = usize::MAX;

            for (byte_idx, _) in &norm_occurrences {
                let line_num = norm_full[..*byte_idx].lines().count() + 1;
                let diff = (line_num as isize - target_line as isize).unsigned_abs();
                if diff < min_diff {
                    min_diff = diff;
                    best_match = Some(*byte_idx);
                }
            }

            if let Some(idx) = best_match {
                let patched = format!("{}{}{}", &norm_full[..idx], norm_new, &norm_full[idx + norm_old.len()..]);
                return Ok(if full_content.contains("\r\n") { patched.replace('\n', "\r\n") } else { patched });
            }
        }
        return Err(format!(
            "Ambigüedad: 'old_text' aparece {} veces en el documento tras normalizar saltos de línea. Especifica 'start_line'.",
            norm_occurrences.len()
        ));
    }

    // 3. Nivel 3: Anclaje difuso por líneas (tolerancia a trailing spaces y tabs)
    let doc_lines: Vec<&str> = norm_full.lines().collect();
    let old_lines: Vec<&str> = norm_old.lines().collect();
    let n_old = old_lines.len();

    if n_old > 0 && n_old <= doc_lines.len() {
        // Intentar primero con .trim_end()
        let mut matches_trim_end = Vec::new();
        for i in 0..=(doc_lines.len() - n_old) {
            let window = &doc_lines[i..i + n_old];
            let all_match = window.iter().zip(&old_lines).all(|(d, o)| d.trim_end() == o.trim_end());
            if all_match {
                matches_trim_end.push(i);
            }
        }

        let chosen_start = if matches_trim_end.len() == 1 {
            Some(matches_trim_end[0])
        } else if matches_trim_end.len() > 1 {
            if let Some(target_line) = start_line_hint {
                matches_trim_end.into_iter().min_by_key(|&line_idx| {
                    (line_idx + 1).abs_diff(target_line)
                })
            } else {
                return Err(format!(
                    "Ambigüedad difusa: se encontraron {} bloques coincidentes ignorando espacios finales. Proporciona 'start_line'.",
                    matches_trim_end.len()
                ));
            }
        } else {
            // Intentar con .trim() por línea (tolerancia de indentación)
            let mut matches_trim = Vec::new();
            for i in 0..=(doc_lines.len() - n_old) {
                let window = &doc_lines[i..i + n_old];
                let all_match = window.iter().zip(&old_lines).all(|(d, o)| d.trim() == o.trim());
                if all_match {
                    matches_trim.push(i);
                }
            }

            if matches_trim.len() == 1 {
                Some(matches_trim[0])
            } else if matches_trim.len() > 1 {
                if let Some(target_line) = start_line_hint {
                    matches_trim.into_iter().min_by_key(|&line_idx| {
                        (line_idx + 1).abs_diff(target_line)
                    })
                } else {
                    return Err(format!(
                        "Ambigüedad difusa: se encontraron {} bloques coincidentes ignorando indentación. Proporciona 'start_line'.",
                        matches_trim.len()
                    ));
                }
            } else {
                None
            }
        };

        if let Some(start_idx) = chosen_start {
            let mut result_lines = Vec::new();
            result_lines.extend_from_slice(&doc_lines[..start_idx]);
            result_lines.push(&norm_new);
            result_lines.extend_from_slice(&doc_lines[start_idx + n_old..]);
            let patched = result_lines.join("\n");
            return Ok(if full_content.contains("\r\n") { patched.replace('\n', "\r\n") } else { patched });
        }
    }

    // 4. Diagnóstico detallado si falló: buscar las líneas más cercanas similares
    let mut closest_preview = String::new();
    if let Some(target_line) = start_line_hint {
        let center = target_line.saturating_sub(1).min(doc_lines.len());
        let start = center.saturating_sub(2);
        let end = (center + 3).min(doc_lines.len());
        if start < end {
            closest_preview = format!(
                "\nLíneas reales cercanas ({}-{}):\n{}",
                start + 1,
                end,
                doc_lines[start..end]
                    .iter()
                    .enumerate()
                    .map(|(i, l)| format!("{}: {}", start + i + 1, l))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }

    Err(format!(
        "No se pudo encontrar 'old_text' en el documento (se intentó coincidencia exacta, normalización CRLF y anclaje difuso por líneas).{}",
        closest_preview
    ))
}

/// Calcula el hash SHA-256 en formato hex con prefijo "sha256:" para control optimista de concurrencia.
pub fn calculate_content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Estima los tokens de texto usando la heurística ceil(bytes / 4).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

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

/// Casos de uso de MCP enriquecidos con búsqueda Full-Text, Grafo/Backlinks, Git y Multimodalidad.
pub struct McpUseCases<R: NoteRepository, I: FullTextIndex + ?Sized, G: GitPort> {
    vault_path: PathBuf,
    note_repository: R,
    full_text_index: Option<Arc<I>>,
    git_service: Option<Arc<G>>,
}

impl<R: NoteRepository, I: FullTextIndex + ?Sized, G: GitPort> McpUseCases<R, I, G> {
    pub fn new(
        vault_path: PathBuf,
        note_repository: R,
        full_text_index: Option<Arc<I>>,
        git_service: Option<Arc<G>>,
    ) -> Self {
        Self {
            vault_path,
            note_repository,
            full_text_index,
            git_service,
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
                    description: "Crea o actualiza una nota completa con protección de hash OCC, freno anti-truncamiento y modo dry_run.".to_string(),
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
                let parent = args.get("parent_path").and_then(|v| v.as_str()).unwrap_or("");
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
    // Implementaciones individuales de Tool calls
    // ==========================================

    fn execute_search_notes(&self, args: &Value) -> McpCallToolResult {
        let query = args.get("query").and_then(|v| v.as_str()).map(|q| q.trim().to_lowercase()).unwrap_or_default();
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let tags_filter: Vec<String> = args
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let extensions = vec!["md".to_string(), "markdown".to_string()];
        match self.note_repository.list_notes(&self.vault_path, &extensions) {
            Ok(notes) => {
                let filtered: Vec<_> = notes
                    .into_iter()
                    .filter(|n| {
                        // Extraer todas las etiquetas (YAML + inline hashtags)
                        let all_tags = tag_service::extract_all_tags(&n.content);

                        let matches_query = query.is_empty()
                            || n.title.to_lowercase().contains(&query)
                            || n.relative_path.as_str().to_lowercase().contains(&query)
                            || all_tags.iter().any(|t| matches_tag(t, &query))
                            || n.frontmatter.as_ref().is_some_and(|fm| {
                                fm.aliases.iter().any(|a| a.to_lowercase().contains(&query))
                            });
                        if !matches_query {
                            return false;
                        }

                        if !tags_filter.is_empty() {
                            // Todas las tags especificadas en tags_filter deben coincidir jerárquicamente
                            let matches_all_tag_filters = tags_filter.iter().all(|tf| {
                                tag_service::any_tag_matches(&all_tags, tf)
                            });
                            if !matches_all_tag_filters {
                                return false;
                            }
                        }
                        true
                    })
                    .take(limit)
                    .map(|n| {
                        let snippet = extract_clean_snippet(&n.content, 200);
                        let total_lines = n.content.lines().count();
                        let est_tokens = estimate_tokens(&n.content);

                        let description = n.frontmatter.as_ref().and_then(|fm| {
                            fm.extra_properties.get("description")
                                .or_else(|| fm.extra_properties.get("summary"))
                                .and_then(|v| v.as_str().map(|s| s.to_string()))
                        });

                        json!({
                            "title": n.title,
                            "relative_path": n.relative_path.as_str(),
                            "tags": n.frontmatter.as_ref().map(|f| &f.tags),
                            "aliases": n.frontmatter.as_ref().map(|f| &f.aliases),
                            "description": description,
                            "snippet": snippet,
                            "total_lines": total_lines,
                            "estimated_tokens": est_tokens,
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

    fn execute_search_folders(&self, args: &Value) -> McpCallToolResult {
        let query = args.get("query").and_then(|v| v.as_str()).map(|q| q.trim().to_lowercase()).unwrap_or_default();
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let extensions = vec!["md".to_string(), "markdown".to_string(), "pdf".to_string(), "excalidraw".to_string()];
        let all_notes = match self.note_repository.list_notes(&self.vault_path, &extensions) {
            Ok(n) => n,
            Err(e) => return McpCallToolResult::error(format!("Error leyendo notas para indexar carpetas: {}", e)),
        };

        // Recolectar todas las carpetas a partir de las notas y del filesystem
        let file_paths: Vec<String> = all_notes.iter().map(|n| n.relative_path.as_str().to_string()).collect();
        let mut folder_set: HashSet<String> = crate::domain::services::folder_service::extract_ancestor_folders(&file_paths)
            .into_iter()
            .collect();

        // Incluir también carpetas vacías descubiertas en disco
        if let Ok(entries) = jwalk::WalkDir::new(&self.vault_path)
            .skip_hidden(true)
            .process_read_dir(|depth, _path, _state, children| {
                if depth.is_none() {
                    return;
                }
                children.retain(|entry_res| {
                    if let Ok(entry) = entry_res {
                        let name = entry.file_name.to_string_lossy();
                        !name.starts_with('.') && name != "target" && name != "node_modules"
                    } else {
                        true
                    }
                });
            })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
        {
            for entry in entries {
                if entry.file_type.is_dir() {
                    if let Ok(rel) = entry.path().strip_prefix(&self.vault_path) {
                        let clean = rel.to_string_lossy().replace('\\', "/").trim_matches('/').to_string();
                        if !clean.is_empty() {
                            folder_set.insert(clean);
                        }
                    }
                }
            }
        }

        let all_folders: Vec<String> = folder_set.into_iter().collect();

        // Preparar información de archivos para resumir huella semántica
        let all_files_vec: Vec<(String, String, Vec<String>)> = all_notes
            .iter()
            .map(|n| {
                let tags = tag_service::extract_all_tags(&n.content);
                (n.relative_path.as_str().to_string(), n.title.clone(), tags)
            })
            .collect();

        let files_ref: Vec<(&str, &str, &[String])> = all_files_vec
            .iter()
            .map(|(p, t, tags)| (p.as_str(), t.as_str(), tags.as_slice()))
            .collect();

        let mut matched: Vec<String> = if query.is_empty() {
            all_folders.clone()
        } else {
            all_folders
                .iter()
                .filter(|f| f.to_lowercase().contains(&query))
                .cloned()
                .collect()
        };

        if !query.is_empty() {
            matched.sort_by(|a, b| {
                let name_a = a.rsplit('/').next().unwrap_or(a).to_lowercase();
                let name_b = b.rsplit('/').next().unwrap_or(b).to_lowercase();
                let exact_a = name_a == query;
                let exact_b = name_b == query;
                if exact_a != exact_b {
                    return if exact_a { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
                }
                let prefix_a = name_a.starts_with(&query);
                let prefix_b = name_b.starts_with(&query);
                if prefix_a != prefix_b {
                    return if prefix_a { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
                }
                a.cmp(b)
            });
        } else {
            matched.sort();
        }

        let total_matched = matched.len();
        let paginated: Vec<_> = matched
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|f| crate::domain::services::folder_service::summarize_folder(&f, &all_folders, &files_ref))
            .collect();

        let has_more = offset + limit < total_matched;
        let next_offset = if has_more { Some(offset + limit) } else { None };

        let payload = json!({
            "total_folders": total_matched,
            "returned_folders": paginated.len(),
            "offset": offset,
            "limit": limit,
            "has_more": has_more,
            "next_offset": next_offset,
            "folders": paginated,
        });

        match serde_json::to_string_pretty(&payload) {
            Ok(text) => McpCallToolResult::text(text),
            Err(e) => McpCallToolResult::error(format!("Error serializando carpetas: {}", e)),
        }
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

        if source_folder.split('/').any(|s| s == "..") || target_folder.split('/').any(|s| s == "..") {
            return McpCallToolResult::error("Ruta no permitida ('..' detectado)");
        }

        if source_folder == target_folder {
            return McpCallToolResult::error("La carpeta de origen y destino no pueden ser la misma");
        }

        // Prevenir bucle infinito: target no puede ser subcarpeta de source
        if target_folder.starts_with(&format!("{}/", source_folder)) {
            return McpCallToolResult::error("La carpeta destino no puede estar dentro de la carpeta origen");
        }

        let extension_filter = args.get("extension").and_then(|v| v.as_str()).map(|e| e.trim_start_matches('.').to_lowercase());
        let conflict_strategy = args.get("conflict_strategy").and_then(|v| v.as_str()).unwrap_or("skip");
        let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

        let source_dir = self.vault_path.join(&source_folder);
        if !source_dir.exists() || !source_dir.is_dir() {
            return McpCallToolResult::error(format!("La carpeta origen '{}' no existe", source_folder));
        }

        let target_dir = self.vault_path.join(&target_folder);

        // Listar todos los archivos dentro de source_folder
        let mut files_to_move = Vec::new();
        if let Ok(entries) = jwalk::WalkDir::new(&source_dir).skip_hidden(true).into_iter().collect::<Result<Vec<_>, _>>() {
            for entry in entries {
                if entry.file_type.is_file() {
                    let path = entry.path();
                    if let Ok(rel) = path.strip_prefix(&source_dir) {
                        let rel_str = rel.to_string_lossy().replace('\\', "/");
                        if let Some(ref ext) = extension_filter {
                            if !rel_str.to_lowercase().ends_with(&format!(".{}", ext)) {
                                continue;
                            }
                        }
                        files_to_move.push(rel_str);
                    }
                }
            }
        }

        if dry_run {
            let payload = json!({
                "dry_run": true,
                "source_folder": source_folder,
                "target_folder": target_folder,
                "total_files_to_move": files_to_move.len(),
                "files": files_to_move,
            });
            return match serde_json::to_string_pretty(&payload) {
                Ok(t) => McpCallToolResult::text(t),
                Err(e) => McpCallToolResult::error(e.to_string()),
            };
        }

        // Crear target_dir si no existe
        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            return McpCallToolResult::error(format!("No se pudo crear la carpeta destino: {}", e));
        }

        let mut moved = Vec::new();
        let mut skipped = Vec::new();
        let mut errors = Vec::new();

        for rel_file in files_to_move {
            let src_file_path = source_dir.join(&rel_file);
            let mut dest_file_path = target_dir.join(&rel_file);

            if let Some(parent) = dest_file_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            if dest_file_path.exists() {
                match conflict_strategy {
                    "skip" => {
                        skipped.push(rel_file);
                        continue;
                    }
                    "overwrite" => {
                        // Proceder con rename/sobrescritura
                    }
                    "rename" => {
                        // Generar nombre no colisionante: archivo_1.ext
                        let stem = dest_file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
                        let ext = dest_file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        let parent = dest_file_path.parent().unwrap();
                        let mut counter = 1;
                        loop {
                            let candidate_name = if ext.is_empty() {
                                format!("{}_{}", stem, counter)
                            } else {
                                format!("{}_{}.{}", stem, counter, ext)
                            };
                            let candidate_path = parent.join(&candidate_name);
                            if !candidate_path.exists() {
                                dest_file_path = candidate_path;
                                break;
                            }
                            counter += 1;
                        }
                    }
                    _ => {
                        skipped.push(rel_file);
                        continue;
                    }
                }
            }

            match std::fs::rename(&src_file_path, &dest_file_path) {
                Ok(_) => {
                    let new_rel = dest_file_path.strip_prefix(&self.vault_path).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
                    moved.push(json!({
                        "original": rel_file,
                        "new_path": new_rel,
                    }));
                }
                Err(e) => {
                    errors.push(format!("Error moviendo '{}': {}", rel_file, e));
                }
            }
        }

        let payload = json!({
            "source_folder": source_folder,
            "target_folder": target_folder,
            "moved_count": moved.len(),
            "skipped_count": skipped.len(),
            "errors_count": errors.len(),
            "moved_files": moved,
            "skipped_files": skipped,
            "errors": errors,
        });

        match serde_json::to_string_pretty(&payload) {
            Ok(t) => McpCallToolResult::text(t),
            Err(e) => McpCallToolResult::error(e.to_string()),
        }
    }

    fn execute_full_text_search(&self, args: &Value) -> McpCallToolResult {
        let query_str = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q.trim(),
            None => return McpCallToolResult::error("El argumento 'query' es obligatorio"),
        };
        if query_str.is_empty() {
            return McpCallToolResult::text("[]");
        }
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let folder_filter = args.get("folder_path").and_then(|v| v.as_str()).map(|f| f.trim_matches('/').to_lowercase());
        let tags_filter: Vec<String> = args
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(ref fts) = self.full_text_index {
            let q = FullTextQuery {
                text: query_str.to_string(),
                limit: limit * 4,
                prefix_last_term: true,
            };
            match fts.search(&q) {
                Ok((hits, total)) => {
                    let mut filtered_hits = Vec::new();
                    for h in hits {
                        if let Some(ref ff) = folder_filter {
                            let lower_path = h.note_path.to_lowercase();
                            let is_prefix = lower_path.starts_with(ff);
                            let is_segment = lower_path.contains(&format!("/{}/", ff))
                                || lower_path.starts_with(&format!("{}/", ff))
                                || lower_path.contains(&format!("/{}", ff));
                            if !is_prefix && !is_segment {
                                continue;
                            }
                        }
                        // Leer la nota para estimar líneas, tokens y verificar filtro de tags si aplica
                        let (total_lines, est_tokens, matches_tags) = if let Ok(rel) = NoteRelativePath::new(&h.note_path) {
                            if let Ok(note) = self.note_repository.read_note(&self.vault_path, &rel) {
                                let lines = note.content.lines().count();
                                let tokens = estimate_tokens(&note.content);
                                let ok_tags = if tags_filter.is_empty() {
                                    true
                                } else {
                                    let all_tags = tag_service::extract_all_tags(&note.content);
                                    tags_filter.iter().all(|tf| tag_service::any_tag_matches(&all_tags, tf))
                                };
                                (lines, tokens, ok_tags)
                            } else {
                                (0, 0, tags_filter.is_empty())
                            }
                        } else {
                            (0, 0, tags_filter.is_empty())
                        };

                        if !matches_tags {
                            continue;
                        }

                        filtered_hits.push(json!({
                            "path": h.note_path,
                            "title": h.title,
                            "score": h.score,
                            "snippet": h.snippet,
                            "total_lines": total_lines,
                            "estimated_tokens": est_tokens,
                            "matched_terms": h.matched_terms,
                        }));

                        if filtered_hits.len() >= limit {
                            break;
                        }
                    }

                    let results = json!({
                        "total_matches": total,
                        "returned_matches": filtered_hits.len(),
                        "hits": filtered_hits,
                    });
                    match serde_json::to_string_pretty(&results) {
                        Ok(text) => McpCallToolResult::text(text),
                        Err(e) => McpCallToolResult::error(format!("Error al serializar: {}", e)),
                    }
                }
                Err(err) => McpCallToolResult::error(format!("Error en full-text search: {}", err)),
            }
        } else {
            McpCallToolResult::error("El índice Tantivy de búsqueda de texto completo no está disponible.")
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
        let context_lines = args.get("context_lines").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
        let context_lines = context_lines.min(10);

        let relative_path = match NoteRelativePath::new(rel_path_str) {
            Ok(p) => p,
            Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
        };

        let note = match self.note_repository.read_note(&self.vault_path, &relative_path) {
            Ok(n) => n,
            Err(e) => return McpCallToolResult::error(format!("Error al leer nota: {}", e)),
        };

        let lines: Vec<&str> = note.content.lines().collect();
        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                let start_idx = i.saturating_sub(context_lines);
                let end_idx = (i + context_lines + 1).min(lines.len());
                let context_snippet: Vec<String> = (start_idx..end_idx)
                    .map(|idx| format!("{}: {}", idx + 1, lines[idx]))
                    .collect();

                matches.push(json!({
                    "line_number": i + 1,
                    "matched_line": line,
                    "context": context_snippet,
                }));
            }
        }

        let payload = json!({
            "relative_path": rel_path_str,
            "total_matches": matches.len(),
            "matches": matches,
        });

        match serde_json::to_string_pretty(&payload) {
            Ok(text) => McpCallToolResult::text(text),
            Err(e) => McpCallToolResult::error(format!("Error al serializar: {}", e)),
        }
    }

    fn execute_read_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        let mut mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");
        let start_line = args.get("start_line").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let line_count = args.get("line_count").and_then(|v| v.as_u64()).unwrap_or(150) as usize;
        let line_count = line_count.min(500);
        let section_heading = args
            .get("section")
            .or_else(|| args.get("section_heading"))
            .and_then(|v| v.as_str());
        let outline_only = args.get("outline_only").and_then(|v| v.as_bool()).unwrap_or(false);
        let include_line_numbers = args.get("include_line_numbers").and_then(|v| v.as_bool()).unwrap_or(false);

        // Si se especificó outline_only o section_heading, inferir el modo automáticamente si mode es el predeterminado
        if outline_only || mode == "outline_only" {
            mode = "metadata_only";
        } else if section_heading.is_some() && mode == "full" {
            mode = "section";
        }

        let relative_path = match NoteRelativePath::new(rel_path_str) {
            Ok(p) => p,
            Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
        };

        match self.note_repository.read_note(&self.vault_path, &relative_path) {
            Ok(note) => {
                let lines: Vec<&str> = note.content.lines().collect();
                let total_lines = lines.len();
                let content_hash = calculate_content_hash(&note.content);
                let est_tokens = estimate_tokens(&note.content);

                // 1. Extraer TOC (Table of Contents)
                let mut toc = Vec::new();
                for (idx, line) in lines.iter().enumerate() {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with('#') {
                        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                        if hashes >= 1 && hashes <= 6 && trimmed.chars().nth(hashes) == Some(' ') {
                            let heading_text = trimmed[hashes..].trim().to_string();
                            toc.push(json!({
                                "level": hashes,
                                "text": heading_text,
                                "line": idx + 1,
                            }));
                        }
                    }
                }

                // 2. Detectar bloques especiales
                let mut mermaid_count = 0;
                let mut d2_count = 0;
                for line in &lines {
                    let trimmed = line.trim();
                    if trimmed.starts_with("```mermaid") { mermaid_count += 1; }
                    if trimmed.starts_with("```d2") { d2_count += 1; }
                }

                let detected_blocks = json!({
                    "mermaid_count": mermaid_count,
                    "d2_count": d2_count,
                });

                match mode {
                    "metadata_only" => {
                        let payload = json!({
                            "relative_path": note.relative_path.as_str(),
                            "title": note.title,
                            "content_hash": content_hash,
                            "total_lines": total_lines,
                            "estimated_tokens": est_tokens,
                            "okf_metadata": note.frontmatter,
                            "toc": toc,
                            "detected_blocks": detected_blocks,
                        });
                        serde_json::to_string_pretty(&payload)
                            .map(McpCallToolResult::text)
                            .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
                    }
                    "section" => {
                        let target_heading = match section_heading {
                            Some(h) => h.trim().trim_start_matches('#').trim().to_lowercase(),
                            None => return McpCallToolResult::error("El argumento 'section' o 'section_heading' es requerido en mode='section'"),
                        };

                        let mut section_start = None;
                        let mut section_level = 1;
                        for (idx, line) in lines.iter().enumerate() {
                            let trimmed = line.trim_start();
                            if trimmed.starts_with('#') {
                                let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                                if hashes >= 1 && hashes <= 6 {
                                    let h_text = trimmed[hashes..].trim().to_lowercase();
                                    if h_text.contains(&target_heading) {
                                        section_start = Some(idx);
                                        section_level = hashes;
                                        break;
                                    }
                                }
                            }
                        }

                        let start_idx = match section_start {
                            Some(s) => s,
                            None => {
                                let available_headings: Vec<String> = toc.iter().filter_map(|h| h.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())).collect();
                                return McpCallToolResult::error(format!(
                                    "Sección '{}' no encontrada en la nota. Secciones disponibles: {:?}",
                                    target_heading, available_headings
                                ));
                            }
                        };

                        let mut end_idx = lines.len();
                        for (idx, line) in lines.iter().enumerate().skip(start_idx + 1) {
                            let trimmed = line.trim_start();
                            if trimmed.starts_with('#') {
                                let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                                if hashes <= section_level {
                                    end_idx = idx;
                                    break;
                                }
                            }
                        }

                        let selected_lines = &lines[start_idx..end_idx];
                        let rendered_content = if include_line_numbers {
                            selected_lines
                                .iter()
                                .enumerate()
                                .map(|(i, l)| format!("{}: {}", start_idx + i + 1, l))
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            selected_lines.join("\n")
                        };

                        let payload = json!({
                            "relative_path": note.relative_path.as_str(),
                            "title": note.title,
                            "content_hash": content_hash,
                            "total_lines": total_lines,
                            "returned_lines": { "start": start_idx + 1, "end": end_idx },
                            "okf_metadata": note.frontmatter,
                            "toc": toc,
                            "content": rendered_content,
                        });
                        serde_json::to_string_pretty(&payload)
                            .map(McpCallToolResult::text)
                            .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
                    }
                    "chunk" => {
                        let start_idx = start_line.saturating_sub(1).min(total_lines);
                        let end_idx = (start_idx + line_count).min(total_lines);
                        let selected_lines = if start_idx < end_idx { &lines[start_idx..end_idx] } else { &[] };

                        let rendered_content = if include_line_numbers {
                            selected_lines
                                .iter()
                                .enumerate()
                                .map(|(i, l)| format!("{}: {}", start_idx + i + 1, l))
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            selected_lines.join("\n")
                        };

                        let payload = json!({
                            "relative_path": note.relative_path.as_str(),
                            "title": note.title,
                            "content_hash": content_hash,
                            "total_lines": total_lines,
                            "returned_lines": { "start": start_idx + 1, "end": end_idx },
                            "has_more": end_idx < total_lines,
                            "okf_metadata": note.frontmatter,
                            "toc": toc,
                            "content": rendered_content,
                        });
                        serde_json::to_string_pretty(&payload)
                            .map(McpCallToolResult::text)
                            .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
                    }
                    _ => {
                        // mode = "full"
                        let rendered_content = if include_line_numbers {
                            lines
                                .iter()
                                .enumerate()
                                .map(|(i, l)| format!("{}: {}", i + 1, l))
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            note.content.clone()
                        };

                        let payload = json!({
                            "relative_path": note.relative_path.as_str(),
                            "title": note.title,
                            "content_hash": content_hash,
                            "total_lines": total_lines,
                            "estimated_tokens": est_tokens,
                            "okf_metadata": note.frontmatter,
                            "toc": toc,
                            "detected_blocks": detected_blocks,
                            "content": rendered_content,
                        });
                        serde_json::to_string_pretty(&payload)
                            .map(McpCallToolResult::text)
                            .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
                    }
                }
            }
            Err(err) => McpCallToolResult::error(format!("Error al leer nota: {}", err)),
        }
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
        let expected_hash = args.get("expected_hash").and_then(|v| v.as_str());
        let allow_large_overwrite = args.get("allow_large_overwrite").and_then(|v| v.as_bool()).unwrap_or(false);
        let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

        let relative_path = match NoteRelativePath::new(rel_path_str) {
            Ok(p) => p,
            Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
        };

        // Leer nota existente si existe
        let existing_note = self.note_repository.read_note(&self.vault_path, &relative_path).ok();

        if let Some(ref note) = existing_note {
            let current_hash = calculate_content_hash(&note.content);
            if let Some(exp) = expected_hash {
                if exp != current_hash {
                    return McpCallToolResult::error(format!(
                        "Conflicto de concurrencia OCC: el archivo cambió en disco. Hash actual: '{}', esperado: '{}'. Relee el documento antes de escribir.",
                        current_hash, exp
                    ));
                }
            }

            // Freno anti-truncamiento (>150 líneas y reduce más del 40%)
            let old_line_count = note.content.lines().count();
            let new_line_count = content.lines().count();
            if old_line_count > 150 && !allow_large_overwrite {
                if new_line_count < (old_line_count * 60) / 100 {
                    return McpCallToolResult::error(format!(
                        "Freno anti-truncamiento activado: la nota original tiene {} líneas y la nueva tiene {}. Usa patch_note para ediciones parciales o establece allow_large_overwrite: true.",
                        old_line_count, new_line_count
                    ));
                }
            }
        }

        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                existing_note
                    .as_ref()
                    .map(|n| n.title.clone())
                    .unwrap_or_else(|| "Sin título".to_string())
            });

        let final_body = if content.trim_start().starts_with("---") {
            content.to_string()
        } else {
            let mut fm = existing_note
                .as_ref()
                .and_then(|n| n.frontmatter.clone())
                .unwrap_or_default();
            fm.title = Some(title.clone());

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

        if dry_run {
            let old_content = existing_note.as_ref().map(|n| n.content.as_str()).unwrap_or("");
            let unified_diff = generate_unified_diff(old_content, &final_body, rel_path_str);
            let payload = json!({
                "dry_run": true,
                "relative_path": rel_path_str,
                "unified_diff": unified_diff,
            });
            return serde_json::to_string_pretty(&payload)
                .map(McpCallToolResult::text)
                .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()));
        }

        let note = Note::with_encoding(
            relative_path,
            "".to_string(),
            title,
            final_body.clone(),
            "UTF-8".to_string(),
        );

        match self.note_repository.save_note(&self.vault_path, &note) {
            Ok(_) => {
                let new_hash = calculate_content_hash(&final_body);
                let payload = json!({
                    "status": "success",
                    "relative_path": rel_path_str,
                    "new_content_hash": new_hash,
                    "message": format!("Nota '{}' guardada exitosamente.", rel_path_str),
                });
                serde_json::to_string_pretty(&payload)
                    .map(McpCallToolResult::text)
                    .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
            }
            Err(err) => McpCallToolResult::error(format!("Error al guardar la nota: {}", err)),
        }
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
        let start_line_hint = args.get("start_line").and_then(|v| v.as_u64()).map(|n| n as usize);
        let expected_hash = args.get("expected_hash").and_then(|v| v.as_str());
        let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

        let relative_path = match NoteRelativePath::new(rel_path_str) {
            Ok(p) => p,
            Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
        };

        let note = match self.note_repository.read_note(&self.vault_path, &relative_path) {
            Ok(n) => n,
            Err(e) => return McpCallToolResult::error(format!("Error al leer nota: {}", e)),
        };

        let current_hash = calculate_content_hash(&note.content);
        if let Some(exp) = expected_hash {
            if exp != current_hash {
                return McpCallToolResult::error(format!(
                    "Conflicto OCC en patch_note: el archivo cambió en disco. Hash actual: '{}', esperado: '{}'. Relee el documento.",
                    current_hash, exp
                ));
            }
        }

        // Algoritmo de reemplazo quirúrgico con tolerancia progresiva:
        let full_content = &note.content;
        let patched_content = match apply_surgical_patch(full_content, old_text, new_text, start_line_hint) {
            Ok(res) => res,
            Err(err_msg) => return McpCallToolResult::error(format!("{} en '{}'", err_msg, rel_path_str)),
        };

        let unified_diff = generate_unified_diff(full_content, &patched_content, rel_path_str);

        if dry_run {
            let payload = json!({
                "dry_run": true,
                "relative_path": rel_path_str,
                "unified_diff": unified_diff,
            });
            return serde_json::to_string_pretty(&payload)
                .map(McpCallToolResult::text)
                .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()));
        }

        let new_hash = calculate_content_hash(&patched_content);
        let updated_note = Note::with_encoding(
            relative_path,
            "".to_string(),
            note.title,
            patched_content,
            note.encoding,
        );

        match self.note_repository.save_note(&self.vault_path, &updated_note) {
            Ok(_) => {
                let payload = json!({
                    "status": "success",
                    "relative_path": rel_path_str,
                    "new_content_hash": new_hash,
                    "unified_diff": unified_diff,
                });
                serde_json::to_string_pretty(&payload)
                    .map(McpCallToolResult::text)
                    .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
            }
            Err(err) => McpCallToolResult::error(format!("Error al guardar parche: {}", err)),
        }
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
        let section_heading = args.get("section_heading").and_then(|v| v.as_str());
        let position = args.get("position").and_then(|v| v.as_str()).unwrap_or("end_of_section");

        let relative_path = match NoteRelativePath::new(rel_path_str) {
            Ok(p) => p,
            Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
        };

        let note = match self.note_repository.read_note(&self.vault_path, &relative_path) {
            Ok(n) => n,
            Err(e) => return McpCallToolResult::error(format!("Error al leer nota: {}", e)),
        };

        let mut lines: Vec<String> = note.content.lines().map(|s| s.to_string()).collect();

        if let Some(heading) = section_heading {
            let target_h = heading.trim().trim_start_matches('#').trim().to_lowercase();
            let mut sec_idx = None;
            let mut sec_level = 1;
            for (idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with('#') {
                    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                    if hashes >= 1 && hashes <= 6 {
                        if trimmed[hashes..].trim().to_lowercase().contains(&target_h) {
                            sec_idx = Some(idx);
                            sec_level = hashes;
                            break;
                        }
                    }
                }
            }

            let start_h = match sec_idx {
                Some(s) => s,
                None => return McpCallToolResult::error(format!("Encabezado de sección '{}' no encontrado", heading)),
            };

            if position == "start_of_section" {
                lines.insert(start_h + 1, content_to_append.to_string());
            } else {
                let mut insert_idx = lines.len();
                for (idx, line) in lines.iter().enumerate().skip(start_h + 1) {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with('#') {
                        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                        if hashes <= sec_level {
                            insert_idx = idx;
                            break;
                        }
                    }
                }
                lines.insert(insert_idx, content_to_append.to_string());
            }
        } else {
            lines.push(content_to_append.to_string());
        }

        let new_content = lines.join("\n");
        let new_hash = calculate_content_hash(&new_content);

        let updated_note = Note::with_encoding(
            relative_path,
            "".to_string(),
            note.title,
            new_content,
            note.encoding,
        );

        match self.note_repository.save_note(&self.vault_path, &updated_note) {
            Ok(_) => {
                let payload = json!({
                    "status": "success",
                    "relative_path": rel_path_str,
                    "new_content_hash": new_hash,
                });
                serde_json::to_string_pretty(&payload)
                    .map(McpCallToolResult::text)
                    .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
            }
            Err(err) => McpCallToolResult::error(format!("Error al anexar contenido: {}", err)),
        }
    }

    fn execute_update_note_metadata(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };

        let relative_path = match NoteRelativePath::new(rel_path_str) {
            Ok(p) => p,
            Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
        };

        let note = match self.note_repository.read_note(&self.vault_path, &relative_path) {
            Ok(n) => n,
            Err(e) => return McpCallToolResult::error(format!("Error al leer nota: {}", e)),
        };

        let (fm_opt, body) = frontmatter_service::parse_frontmatter_and_content(&note.content);
        let mut fm = fm_opt.unwrap_or_default();

        if let Some(new_title) = args.get("title").and_then(|v| v.as_str()) {
            fm.title = Some(new_title.to_string());
        }

        if let Some(add_tags) = args.get("add_tags").and_then(|v| v.as_array()) {
            let mut tag_set: HashSet<String> = fm.tags.into_iter().collect();
            for t in add_tags.iter().filter_map(|t| t.as_str()) {
                tag_set.insert(t.to_string());
            }
            fm.tags = tag_set.into_iter().collect();
            fm.tags.sort();
        }

        if let Some(remove_tags) = args.get("remove_tags").and_then(|v| v.as_array()) {
            let remove_set: HashSet<&str> = remove_tags.iter().filter_map(|t| t.as_str()).collect();
            fm.tags.retain(|t| !remove_set.contains(t.as_str()));
        }

        if let Some(aliases_val) = args.get("aliases").and_then(|v| v.as_array()) {
            fm.aliases = aliases_val.iter().filter_map(|a| a.as_str().map(|s| s.to_string())).collect();
        }

        let new_full_content = frontmatter_service::serialize_frontmatter_and_content(Some(&fm), &body);
        let new_hash = calculate_content_hash(&new_full_content);

        let final_title = fm.title.clone().unwrap_or(note.title);
        let updated_note = Note::with_encoding(
            relative_path,
            "".to_string(),
            final_title,
            new_full_content,
            note.encoding,
        );

        match self.note_repository.save_note(&self.vault_path, &updated_note) {
            Ok(_) => {
                let payload = json!({
                    "status": "success",
                    "relative_path": rel_path_str,
                    "updated_metadata": fm,
                    "new_content_hash": new_hash,
                });
                serde_json::to_string_pretty(&payload)
                    .map(McpCallToolResult::text)
                    .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
            }
            Err(err) => McpCallToolResult::error(format!("Error al actualizar metadatos: {}", err)),
        }
    }

    fn execute_list_folder_contents_tool(&self, args: &Value) -> McpCallToolResult {
        let folder = args.get("folder_path").and_then(|v| v.as_str()).unwrap_or("");
        let include_notes = args.get("include_notes").and_then(|v| v.as_bool()).unwrap_or(true);
        let include_folders = args.get("include_folders").and_then(|v| v.as_bool()).unwrap_or(true);
        let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50).clamp(1, 200) as usize;
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        self.execute_list_folder_contents(folder, include_notes, include_folders, recursive, None, Some(limit), Some(offset))
    }

    fn execute_list_sibling_notes(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        if rel_path_str.is_empty() {
            return McpCallToolResult::error("El argumento 'relative_path' no puede estar vacío");
        }

        let include_self = args.get("include_self").and_then(|v| v.as_bool()).unwrap_or(false);
        let include_folders = args.get("include_folders").and_then(|v| v.as_bool()).unwrap_or(false);

        let parent_folder = match rel_path_str.rsplit_once('/') {
            Some((dir, _)) => dir,
            None => "",
        };

        let exclude_path = if include_self {
            None
        } else {
            Some(rel_path_str.as_str())
        };

        self.execute_list_folder_contents(parent_folder, true, include_folders, false, exclude_path, None, None)
    }

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
        if clean_folder.split('/').any(|seg| seg == "..") {
            return McpCallToolResult::error("Ruta de carpeta no permitida ('..' detectado)");
        }

        let target_dir = if clean_folder.is_empty() {
            self.vault_path.clone()
        } else {
            self.vault_path.join(&clean_folder)
        };

        if !target_dir.exists() || !target_dir.is_dir() {
            return McpCallToolResult::error(format!("La carpeta '{}' no existe o no es un directorio", clean_folder));
        }

        let mut folders_out = Vec::new();
        let mut notes_out = Vec::new();

        if !recursive {
            if let Ok(entries) = std::fs::read_dir(&target_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    let rel_path = if clean_folder.is_empty() {
                        name.clone()
                    } else {
                        format!("{}/{}", clean_folder, name)
                    };

                    if is_dir {
                        if include_folders {
                            folders_out.push(json!({
                                "name": name,
                                "relative_path": rel_path,
                            }));
                        }
                    } else if include_notes {
                        if let Some(ex) = exclude_note_path {
                            if rel_path == ex {
                                continue;
                            }
                        }

                        let line_count = if let Ok(rel) = NoteRelativePath::new(&rel_path) {
                            self.note_repository.read_note(&self.vault_path, &rel).map(|n| n.content.lines().count()).unwrap_or(0)
                        } else {
                            0
                        };

                        let note_title = if let Ok(rel) = NoteRelativePath::new(&rel_path) {
                            self.note_repository.read_note(&self.vault_path, &rel).map(|n| n.title).unwrap_or_else(|_| name.clone())
                        } else {
                            name.clone()
                        };
                        notes_out.push(json!({
                            "name": name,
                            "title": note_title,
                            "relative_path": rel_path,
                            "line_count": line_count,
                        }));
                    }
                }
            }
        } else {
            let extensions = vec!["md".to_string(), "markdown".to_string()];
            if include_notes {
                if let Ok(all_notes) = self.note_repository.list_notes(&self.vault_path, &extensions) {
                    for n in all_notes {
                        let path_str = n.relative_path.as_str();
                        if clean_folder.is_empty() || path_str.starts_with(&format!("{}/", clean_folder)) {
                            if let Some(ex) = exclude_note_path {
                                if path_str == ex {
                                    continue;
                                }
                            }
                            let name = path_str.rsplit('/').next().unwrap_or(path_str).to_string();
                            notes_out.push(json!({
                                "name": name,
                                "relative_path": path_str,
                                "title": n.title,
                            }));
                        }
                    }
                }
            }
        }

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

        match serde_json::to_string_pretty(&payload) {
            Ok(text) => McpCallToolResult::text(text),
            Err(e) => McpCallToolResult::error(format!("Error al serializar contenidos: {}", e)),
        }
    }

    fn execute_move_notes_tool(&self, args: &Value) -> McpCallToolResult {
        let notes_val = match args.get("notes").and_then(|v| v.as_array()) {
            Some(n) => n,
            None => return McpCallToolResult::error("El argumento 'notes' debe ser una lista de rutas"),
        };
        let target_folder = match args.get("target_folder").and_then(|v| v.as_str()) {
            Some(tf) => tf,
            None => return McpCallToolResult::error("El argumento 'target_folder' es obligatorio"),
        };
        let create_target_folder = args.get("create_target_folder").and_then(|v| v.as_bool()).unwrap_or(true);
        let overwrite = args.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(false);
        let update_wikilinks = args.get("update_wikilinks").and_then(|v| v.as_bool()).unwrap_or(false);

        let note_paths: Vec<String> = notes_val
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        self.execute_move_notes(&note_paths, target_folder, create_target_folder, overwrite, update_wikilinks)
    }

    fn execute_move_notes(
        &self,
        notes: &[String],
        target_folder: &str,
        create_target_folder: bool,
        overwrite: bool,
        update_wikilinks: bool,
    ) -> McpCallToolResult {
        let clean_target = target_folder.trim_matches('/').replace('\\', "/");
        if clean_target.split('/').any(|seg| seg == "..") {
            return McpCallToolResult::error("Ruta de destino no permitida ('..' detectado)");
        }

        let dest_dir = if clean_target.is_empty() {
            self.vault_path.clone()
        } else {
            self.vault_path.join(&clean_target)
        };

        if !dest_dir.exists() {
            if create_target_folder {
                if let Err(e) = std::fs::create_dir_all(&dest_dir) {
                    return McpCallToolResult::error(format!("Error al crear carpeta de destino: {}", e));
                }
            } else {
                return McpCallToolResult::error(format!("La carpeta destino '{}' no existe", clean_target));
            }
        }

        let mut moved_success = Vec::new();
        let mut errors = Vec::new();

        for source_rel in notes {
            let clean_src = source_rel.trim_matches('/').replace('\\', "/");
            let file_name = match clean_src.rsplit('/').next() {
                Some(f) => f,
                None => {
                    errors.push(json!({ "source": source_rel, "error": "Ruta inválida" }));
                    continue;
                }
            };

            let src_path = self.vault_path.join(&clean_src);
            let target_rel = if clean_target.is_empty() {
                file_name.to_string()
            } else {
                format!("{}/{}", clean_target, file_name)
            };
            let dst_path = self.vault_path.join(&target_rel);

            if !src_path.exists() {
                errors.push(json!({ "source": source_rel, "error": "Archivo origen no existe" }));
                continue;
            }

            if dst_path.exists() && !overwrite {
                errors.push(json!({ "source": source_rel, "error": "Ya existe un archivo en destino" }));
                continue;
            }

            match std::fs::rename(&src_path, &dst_path) {
                Ok(_) => {
                    moved_success.push(json!({
                        "source": clean_src,
                        "destination": target_rel,
                    }));

                    if update_wikilinks {
                        // Reescritura simple de wikilinks para este archivo
                        let old_stem = file_name.rsplit_once('.').map(|(s, _)| s).unwrap_or(file_name);
                        let extensions = vec!["md".to_string(), "markdown".to_string()];
                        if let Ok(all_notes) = self.note_repository.list_notes(&self.vault_path, &extensions) {
                            for other in all_notes {
                                if other.relative_path.as_str() == target_rel { continue; }
                                if let Ok(mut note_to_patch) = self.note_repository.read_note(&self.vault_path, &other.relative_path) {
                                    if note_to_patch.content.contains(&format!("[[{}]]", old_stem)) || note_to_patch.content.contains(&format!("[[{}]]", clean_src)) {
                                        let updated = note_to_patch.content
                                            .replace(&format!("[[{}]]", clean_src), &format!("[[{}]]", target_rel))
                                            .replace(&format!("[[{}]]", old_stem), &format!("[[{}]]", target_rel));
                                        note_to_patch.content = updated;
                                        let _ = self.note_repository.save_note(&self.vault_path, &note_to_patch);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    errors.push(json!({ "source": source_rel, "error": e.to_string() }));
                }
            }
        }

        let payload = json!({
            "moved_count": moved_success.len(),
            "moved": moved_success,
            "error_count": errors.len(),
            "errors_count": errors.len(),
            "errors": errors,
        });

        match serde_json::to_string_pretty(&payload) {
            Ok(text) => McpCallToolResult::text(text),
            Err(e) => McpCallToolResult::error(format!("Error al serializar move_notes: {}", e)),
        }
    }

    fn execute_delete_note(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p.trim_matches('/').replace('\\', "/"),
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };

        if rel_path_str.is_empty() {
            return McpCallToolResult::error("El argumento 'relative_path' no puede estar vacío");
        }

        let note_path = match NoteRelativePath::new(&rel_path_str) {
            Ok(np) => np,
            Err(err) => return McpCallToolResult::error(err),
        };

        // Si se especificó expected_hash, verificar OCC antes de borrar
        if let Some(expected_hash) = args.get("expected_hash").and_then(|v| v.as_str()) {
            let note = match self.note_repository.read_note(&self.vault_path, &note_path) {
                Ok(n) => n,
                Err(e) => return McpCallToolResult::error(format!("No se pudo leer la nota para validar el hash: {}", e)),
            };

            let current_hash = calculate_content_hash(&note.content);
            if current_hash != expected_hash {
                return McpCallToolResult::error(format!(
                    "Conflicto de concurrencia optimista (OCC): el hash esperado '{}' no coincide con el hash actual de la nota '{}'",
                    expected_hash, current_hash
                ));
            }
        }

        match self.note_repository.delete_note(&self.vault_path, &note_path) {
            Ok(()) => {
                let payload = json!({
                    "status": "success",
                    "deleted_path": rel_path_str,
                    "message": "Nota eliminada correctamente"
                });
                serde_json::to_string_pretty(&payload)
                    .map(McpCallToolResult::text)
                    .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
            }
            Err(err) => McpCallToolResult::error(format!("Error al eliminar nota: {}", err)),
        }
    }

    fn execute_get_backlinks_and_relations(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };

        let extensions = vec!["md".to_string(), "markdown".to_string()];
        let all_notes = match self.note_repository.list_notes(&self.vault_path, &extensions) {
            Ok(notes) => notes,
            Err(e) => return McpCallToolResult::error(format!("Error al listar notas: {}", e)),
        };

        let target_normalized = rel_path_str.trim_start_matches('/').replace('\\', "/");
        let target_stem = target_normalized
            .rsplit('/')
            .next()
            .and_then(|n| n.rsplit_once('.').map(|(s, _)| s))
            .unwrap_or(&target_normalized);

        let mut target_tags: HashSet<String> = HashSet::new();
        let mut backlinks = Vec::new();
        let mut outlinks = Vec::new();
        let mut all_note_tags: HashMap<String, HashSet<String>> = HashMap::new();
        let all_note_paths: HashSet<String> = all_notes.iter().map(|n| n.relative_path.as_str().to_string()).collect();

        for note in &all_notes {
            let n_path = note.relative_path.as_str();
            let full_note = match self.note_repository.read_note(&self.vault_path, &note.relative_path) {
                Ok(n) => n,
                Err(_) => continue,
            };

            let note_tags_set: HashSet<String> = full_note
                .frontmatter
                .as_ref()
                .map(|fm| fm.tags.iter().map(|t| t.to_lowercase()).collect())
                .unwrap_or_default();

            if n_path == target_normalized {
                target_tags = note_tags_set.clone();
            }
            all_note_tags.insert(n_path.to_string(), note_tags_set);

            let wikilinks = extract_wikilinks(&full_note.content);
            let mut links_to_target = false;
            let mut matching_snippet = String::new();

            for wl in wikilinks {
                let (norm_target, _) = normalize_link_target(&wl.target);
                if norm_target == target_normalized
                    || norm_target == format!("{}.md", target_stem)
                    || wl.target.eq_ignore_ascii_case(target_stem)
                {
                    links_to_target = true;
                    // Extraer snippet de contexto de la línea
                    for line in full_note.content.lines() {
                        if line.contains(&format!("[[{}]]", wl.target)) || line.contains(&wl.target) {
                            matching_snippet = line.trim().to_string();
                            break;
                        }
                    }
                }

                if n_path == target_normalized {
                    let is_resolved = all_note_paths.contains(&norm_target) || all_note_paths.contains(&format!("{}.md", norm_target));
                    outlinks.push(json!({
                        "target": wl.target,
                        "alias": wl.alias,
                        "is_embed": wl.is_embed,
                        "is_resolved": is_resolved,
                    }));
                }
            }

            if links_to_target && n_path != target_normalized {
                backlinks.push(json!({
                    "source_path": n_path,
                    "title": note.title,
                    "context_snippet": matching_snippet,
                }));
            }
        }

        // Similitud Jaccard de etiquetas (Top 5 vecinos)
        let mut jaccard_neighbors = Vec::new();
        if !target_tags.is_empty() {
            for (other_path, other_tags) in &all_note_tags {
                if other_path == &target_normalized || other_tags.is_empty() {
                    continue;
                }
                let intersection = target_tags.intersection(other_tags).count();
                let union = target_tags.union(other_tags).count();
                if union > 0 && intersection > 0 {
                    let score = (intersection as f64) / (union as f64);
                    jaccard_neighbors.push((other_path.clone(), score));
                }
            }
            jaccard_neighbors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            jaccard_neighbors.truncate(5);
        }

        let neighbors_json: Vec<_> = jaccard_neighbors
            .into_iter()
            .map(|(p, s)| json!({ "path": p, "jaccard_score": (s * 100.0).round() / 100.0 }))
            .collect();

        let payload = json!({
            "target_note": target_normalized,
            "backlinks_count": backlinks.len(),
            "backlinks": backlinks,
            "outlinks_count": outlinks.len(),
            "outlinks": outlinks,
            "shared_tags_neighbors": neighbors_json,
        });

        match serde_json::to_string_pretty(&payload) {
            Ok(text) => McpCallToolResult::text(text),
            Err(e) => McpCallToolResult::error(format!("Error al serializar: {}", e)),
        }
    }

    fn execute_get_vault_git_status(&self) -> McpCallToolResult {
        if let Some(ref git) = self.git_service {
            match git.get_vault_status(&self.vault_path) {
                Ok(status) => match serde_json::to_string_pretty(&status) {
                    Ok(text) => McpCallToolResult::text(text),
                    Err(e) => McpCallToolResult::error(format!("Error al serializar: {}", e)),
                },
                Err(err) => McpCallToolResult::error(format!("Error al consultar git: {}", err)),
            }
        } else {
            McpCallToolResult::error("El servicio Git no está disponible o la carpeta no es un repositorio.")
        }
    }

    fn execute_read_pdf_document(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("outline_only");
        let start_page = args.get("start_page").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let end_page = args.get("end_page").and_then(|v| v.as_u64()).unwrap_or(start_page as u64 + 4) as usize;
        let end_page = end_page.min(start_page + 4); // Límite estricto: máximo 5 páginas
        let search_query = args.get("search_query").and_then(|v| v.as_str());

        let pdf_path = self.vault_path.join(rel_path_str);
        if !pdf_path.exists() {
            return McpCallToolResult::error(format!("El archivo PDF '{}' no existe", rel_path_str));
        }

        let bytes = match std::fs::read(&pdf_path) {
            Ok(b) => b,
            Err(e) => return McpCallToolResult::error(format!("Error al leer PDF: {}", e)),
        };

        match mode {
            "pages" => {
                match pdf_extractor::extract_pdf_page_range(&bytes, start_page, end_page, 20_000) {
                    Ok(res) => serde_json::to_string_pretty(&res)
                        .map(McpCallToolResult::text)
                        .unwrap_or_else(|e| McpCallToolResult::error(e.to_string())),
                    Err(err) => McpCallToolResult::error(err),
                }
            }
            "search" => {
                let query = match search_query {
                    Some(q) => q,
                    None => return McpCallToolResult::error("El argumento 'search_query' es obligatorio en mode='search'"),
                };
                match pdf_extractor::search_pdf_content(&bytes, query, 20) {
                    Ok(matches) => {
                        let payload = json!({
                            "relative_path": rel_path_str,
                            "query": query,
                            "matches_count": matches.len(),
                            "matches": matches,
                        });
                        serde_json::to_string_pretty(&payload)
                            .map(McpCallToolResult::text)
                            .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
                    }
                    Err(err) => McpCallToolResult::error(err),
                }
            }
            _ => {
                // "outline_only"
                match pdf_extractor::extract_pdf_page_range(&bytes, 1, 1, 100) {
                    Ok(res) => {
                        let payload = json!({
                            "relative_path": rel_path_str,
                            "total_pages": res.total_pages,
                            "file_size_bytes": bytes.len(),
                        });
                        serde_json::to_string_pretty(&payload)
                            .map(McpCallToolResult::text)
                            .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()))
                    }
                    Err(err) => McpCallToolResult::error(err),
                }
            }
        }
    }

    fn execute_extract_note_diagrams(&self, args: &Value) -> McpCallToolResult {
        let rel_path_str = match args.get("relative_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return McpCallToolResult::error("El argumento 'relative_path' es obligatorio"),
        };
        let diagram_type = args.get("diagram_type").and_then(|v| v.as_str()).unwrap_or("all");

        let relative_path = match NoteRelativePath::new(rel_path_str) {
            Ok(p) => p,
            Err(e) => return McpCallToolResult::error(format!("Ruta inválida: {}", e)),
        };

        // Si es archivo directo .d2 o .mmd
        if rel_path_str.ends_with(".d2") || rel_path_str.ends_with(".mmd") {
            let full_path = self.vault_path.join(rel_path_str);
            let content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(e) => return McpCallToolResult::error(format!("Error al leer archivo de diagrama: {}", e)),
            };
            let kind = if rel_path_str.ends_with(".d2") { "d2" } else { "mermaid" };
            let payload = json!({
                "relative_path": rel_path_str,
                "is_dedicated_file": true,
                "diagrams": [{
                    "type": kind,
                    "start_line": 1,
                    "end_line": content.lines().count(),
                    "code": content,
                }],
            });
            return serde_json::to_string_pretty(&payload)
                .map(McpCallToolResult::text)
                .unwrap_or_else(|e| McpCallToolResult::error(e.to_string()));
        }

        let note = match self.note_repository.read_note(&self.vault_path, &relative_path) {
            Ok(n) => n,
            Err(e) => return McpCallToolResult::error(format!("Error al leer nota: {}", e)),
        };

        let lines: Vec<&str> = note.content.lines().collect();
        let mut diagrams = Vec::new();
        let mut in_block = false;
        let mut current_type = "";
        let mut start_l = 0;
        let mut block_lines = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !in_block {
                if trimmed.starts_with("```mermaid") {
                    in_block = true;
                    current_type = "mermaid";
                    start_l = idx + 1;
                    block_lines.clear();
                } else if trimmed.starts_with("```d2") {
                    in_block = true;
                    current_type = "d2";
                    start_l = idx + 1;
                    block_lines.clear();
                }
            } else if trimmed.starts_with("```") {
                in_block = false;
                let matches_filter = diagram_type == "all" || diagram_type == current_type;
                if matches_filter {
                    diagrams.push(json!({
                        "type": current_type,
                        "start_line": start_l,
                        "end_line": idx + 1,
                        "code": block_lines.join("\n"),
                    }));
                }
            } else {
                block_lines.push(*line);
            }
        }

        let payload = json!({
            "relative_path": rel_path_str,
            "total_diagrams": diagrams.len(),
            "diagrams": diagrams,
        });

        match serde_json::to_string_pretty(&payload) {
            Ok(text) => McpCallToolResult::text(text),
            Err(e) => McpCallToolResult::error(format!("Error al serializar diagramas: {}", e)),
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
