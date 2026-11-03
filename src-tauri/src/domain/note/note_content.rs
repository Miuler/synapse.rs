//! Reglas sobre el texto de una nota: huella para control de concurrencia, tamaño estimado,
//! fragmento de resumen, índice de encabezados, secciones, diagramas y parches de texto.
use crate::domain::note::frontmatter::NoteFrontmatter;
use sha2::{Digest, Sha256};

/// Encabezado Markdown de una nota.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: usize,
    pub text: String,
    /// Línea (base 1).
    pub line: usize,
}

/// Bloque de diagrama (` ```mermaid ` o ` ```d2 `) dentro de una nota.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramBlock {
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub code: String,
}

/// Texto completo de una nota.
pub struct NoteContent;

impl NoteContent {
    /// Extrae un fragmento de texto limpio (sin frontmatter YAML, sin encabezados Markdown vacíos) para resumen o snippet.
    pub fn clean_snippet(content: &str, max_chars: usize) -> String {
        let (_, body) = NoteFrontmatter::parse(content);
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
    pub fn apply_patch(
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
            return Ok(format!(
                "{}{}{}",
                &full_content[..idx],
                new_text,
                &full_content[idx + old_text.len()..]
            ));
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
                    return Ok(format!(
                        "{}{}{}",
                        &full_content[..idx],
                        new_text,
                        &full_content[idx + old_text.len()..]
                    ));
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
            let patched = format!(
                "{}{}{}",
                &norm_full[..idx],
                norm_new,
                &norm_full[idx + norm_old.len()..]
            );
            // Si el archivo original usaba predominantemente CRLF, restaurar \r\n
            return Ok(if full_content.contains("\r\n") {
                patched.replace('\n', "\r\n")
            } else {
                patched
            });
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
                    let patched = format!(
                        "{}{}{}",
                        &norm_full[..idx],
                        norm_new,
                        &norm_full[idx + norm_old.len()..]
                    );
                    return Ok(if full_content.contains("\r\n") {
                        patched.replace('\n', "\r\n")
                    } else {
                        patched
                    });
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
                let all_match = window
                    .iter()
                    .zip(&old_lines)
                    .all(|(d, o)| d.trim_end() == o.trim_end());
                if all_match {
                    matches_trim_end.push(i);
                }
            }

            let chosen_start = if matches_trim_end.len() == 1 {
                Some(matches_trim_end[0])
            } else if matches_trim_end.len() > 1 {
                if let Some(target_line) = start_line_hint {
                    matches_trim_end
                        .into_iter()
                        .min_by_key(|&line_idx| (line_idx + 1).abs_diff(target_line))
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
                    let all_match = window
                        .iter()
                        .zip(&old_lines)
                        .all(|(d, o)| d.trim() == o.trim());
                    if all_match {
                        matches_trim.push(i);
                    }
                }

                if matches_trim.len() == 1 {
                    Some(matches_trim[0])
                } else if matches_trim.len() > 1 {
                    if let Some(target_line) = start_line_hint {
                        matches_trim
                            .into_iter()
                            .min_by_key(|&line_idx| (line_idx + 1).abs_diff(target_line))
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
                return Ok(if full_content.contains("\r\n") {
                    patched.replace('\n', "\r\n")
                } else {
                    patched
                });
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
    pub fn hash(text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("sha256:{:x}", hasher.finalize())
    }
    /// Estima los tokens de texto usando la heurística ceil(bytes / 4).
    pub fn estimate_tokens(text: &str) -> usize {
        (text.len() + 3) / 4
    }

    /// Índice de encabezados (`#` a `######` seguidos de espacio).
    pub fn toc(lines: &[&str]) -> Vec<Heading> {
        let mut toc = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                if hashes >= 1 && hashes <= 6 && trimmed.chars().nth(hashes) == Some(' ') {
                    toc.push(Heading {
                        level: hashes,
                        text: trimmed[hashes..].trim().to_string(),
                        line: idx + 1,
                    });
                }
            }
        }
        toc
    }

    /// Cantidad de bloques ` ```mermaid ` y ` ```d2 `.
    pub fn diagram_block_counts(lines: &[&str]) -> (usize, usize) {
        let mut mermaid_count = 0;
        let mut d2_count = 0;
        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("```mermaid") {
                mermaid_count += 1;
            }
            if trimmed.starts_with("```d2") {
                d2_count += 1;
            }
        }
        (mermaid_count, d2_count)
    }

    /// Primer encabezado cuyo texto contiene `target_heading` (en minúsculas): índice de línea
    /// (base 0) y nivel.
    pub fn find_heading(lines: &[&str], target_heading: &str) -> Option<(usize, usize)> {
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                if hashes >= 1
                    && hashes <= 6
                    && trimmed[hashes..]
                        .trim()
                        .to_lowercase()
                        .contains(target_heading)
                {
                    return Some((idx, hashes));
                }
            }
        }
        None
    }

    /// Fin (exclusivo, base 0) de la sección que empieza en `start_idx` con nivel `level`: el
    /// siguiente encabezado de nivel igual o superior, o el final.
    pub fn section_end(lines: &[&str], start_idx: usize, level: usize) -> usize {
        for (idx, line) in lines.iter().enumerate().skip(start_idx + 1) {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                let hashes = trimmed.chars().take_while(|&c| c == '#').count();
                if hashes <= level {
                    return idx;
                }
            }
        }
        lines.len()
    }

    /// Bloques de diagrama de la nota, filtrados por tipo (`all`, `mermaid` o `d2`).
    pub fn diagrams(lines: &[&str], diagram_type: &str) -> Vec<DiagramBlock> {
        let mut diagrams = Vec::new();
        let mut in_block = false;
        let mut current_type = "";
        let mut start_l = 0;
        let mut block_lines: Vec<&str> = Vec::new();

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
                if diagram_type == "all" || diagram_type == current_type {
                    diagrams.push(DiagramBlock {
                        kind: current_type.to_string(),
                        start_line: start_l,
                        end_line: idx + 1,
                        code: block_lines.join("\n"),
                    });
                }
            } else {
                block_lines.push(*line);
            }
        }
        diagrams
    }
}
