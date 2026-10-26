/// Servicio de dominio para extracción y normalización de etiquetas (#tags).
/// Soporta tags en el frontmatter (YAML) y hashtags inline (#etiqueta) en el cuerpo Markdown.

use pulldown_cmark::{Event, Parser};
use std::collections::HashSet;

/// Extrae todas las etiquetas únicas presentes en el contenido de una nota Markdown.
/// Combina las etiquetas definidas en el frontmatter YAML con los hashtags inline del cuerpo.
pub fn extract_all_tags(content: &str) -> Vec<String> {
    let mut tags_set = HashSet::new();

    // 1. Extraer tags del frontmatter YAML si existe
    let (fm, body) = crate::domain::services::frontmatter_service::parse_frontmatter_and_content(content);
    if let Some(fm) = fm {
        for tag in fm.tags {
            let normalized = normalize_tag(&tag);
            if !normalized.is_empty() {
                tags_set.insert(normalized);
            }
        }
    }

    // 2. Extraer hashtags inline del cuerpo markdown (omitiendo bloques de código)
    let inline_tags = extract_inline_hashtags(&body);
    for tag in inline_tags {
        tags_set.insert(tag);
    }

    let mut result: Vec<String> = tags_set.into_iter().collect();
    result.sort();
    result
}

/// Normaliza un tag: elimina '#' inicial, espacios y caracteres no deseados.
pub fn normalize_tag(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_hash = trimmed.strip_prefix('#').unwrap_or(trimmed);
    without_hash
        .trim_matches(|c: char| c.is_whitespace() || c == '/' || c == ',')
        .to_string()
}

/// Extrae hashtags inline del markdown (`#etiqueta`, `#categoria/subcategoria`).
/// Ignora encabezados Markdown (# Heading), bloques de código `inline` y cercados ````.
pub fn extract_inline_hashtags(markdown: &str) -> Vec<String> {
    use pulldown_cmark::Tag;
    let mut tags = Vec::new();
    let parser = Parser::new(markdown);
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::Text(text) if !in_code_block => {
                let text_str = text.as_ref();
                let chars: Vec<char> = text_str.chars().collect();
                let len = chars.len();
                let mut i = 0;

                while i < len {
                    if chars[i] == '#' {
                        // Comprobar que no sea parte de una palabra previa (ej. abc#123 no es un tag)
                        let preceded_by_whitespace_or_punct = if i == 0 {
                            true
                        } else {
                            let prev = chars[i - 1];
                            prev.is_whitespace() || prev == '(' || prev == '[' || prev == '{' || prev == '"' || prev == '\''
                        };

                        if preceded_by_whitespace_or_punct && i + 1 < len {
                            let next = chars[i + 1];
                            // Un hashtag válido debe comenzar con una letra, dígito o guión bajo (no espacio ni otro '#')
                            if next.is_alphanumeric() || next == '_' {
                                let mut j = i + 1;
                                while j < len {
                                    let c = chars[j];
                                    if c.is_alphanumeric() || c == '_' || c == '-' || c == '/' {
                                        j += 1;
                                    } else {
                                        break;
                                    }
                                }

                                let tag_candidate: String = chars[i + 1..j].iter().collect();
                                let normalized = normalize_tag(&tag_candidate);
                                // Un hashtag no puede ser solamente números puros (ej. #123 suele ser número de issue)
                                if !normalized.is_empty() && normalized.chars().any(|c| c.is_alphabetic()) {
                                    tags.push(normalized);
                                }
                                i = j;
                                continue;
                            }
                        }
                    }
                    i += 1;
                }
            }
            _ => {}
        }
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_inline_hashtags_basic() {
        let md = "Este es un texto con #rust y #web/svelte5 como tags. También #arquitectura_limpia.";
        let tags = extract_inline_hashtags(md);
        assert_eq!(tags, vec!["rust", "web/svelte5", "arquitectura_limpia"]);
    }

    #[test]
    fn test_ignore_markdown_headings_and_code() {
        let md = "# Encabezado H1\n\nTexto con `código inline #no_tag` y un tag real #activo.\n\n```rust\n// #tampoco_aqui\nlet x = 1;\n```";
        let tags = extract_inline_hashtags(md);
        assert_eq!(tags, vec!["activo"]);
    }

    #[test]
    fn test_extract_all_tags_with_frontmatter() {
        let note = r#"---
title: Mi Nota
tags:
  - backend
  - rust
---
# Titulo Principal
Contenido con #frontend y #rust repetido.
"#;
        let tags = extract_all_tags(note);
        assert_eq!(tags, vec!["backend", "frontend", "rust"]);
    }
}
