use crate::domain::models::frontmatter::NoteFrontmatter;

/// Separa el contenido raw de una nota Markdown en (Frontmatter opcional, Contenido Markdown restante).
///
/// Soporta formato estándar YAML delimitado por `---` al principio del archivo.
pub fn parse_frontmatter_and_content(raw: &str) -> (Option<NoteFrontmatter>, String) {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return (None, raw.to_string());
    }

    // El primer delimitador debe ser exactamente `---` seguido de salto de línea
    let after_first = &trimmed[3..];
    let content_start = match after_first.strip_prefix("\r\n") {
        Some(rest) => rest,
        None => match after_first.strip_prefix('\n') {
            Some(rest) => rest,
            None => return (None, raw.to_string()),
        },
    };

    // Buscar el delimitador de cierre `---` en su propia línea
    let mut closing_idx = None;
    let mut current_offset = 0;

    for line in content_start.split_inclusive('\n') {
        let trimmed_line = line.trim_end_matches(['\r', '\n']);
        if trimmed_line == "---" {
            closing_idx = Some(current_offset);
            break;
        }
        current_offset += line.len();
    }

    let Some(closing_pos) = closing_idx else {
        return (None, raw.to_string());
    };

    let yaml_slice = &content_start[..closing_pos];
    let after_closing = &content_start[closing_pos..];

    // Saltar el delimitador `---` y su salto de línea
    let body_start = if let Some(rest) = after_closing.strip_prefix("---\r\n") {
        rest
    } else if let Some(rest) = after_closing.strip_prefix("---\n") {
        rest
    } else if let Some(rest) = after_closing.strip_prefix("---") {
        rest
    } else {
        after_closing
    };

    // Intentar deserializar el bloque YAML
    let frontmatter = match serde_yaml::from_str::<NoteFrontmatter>(yaml_slice) {
        Ok(fm) => {
            if fm.is_empty() {
                None
            } else {
                Some(fm)
            }
        }
        Err(err) => {
            log::warn!("No se pudo deserializar el frontmatter YAML: {}", err);
            None
        }
    };

    (frontmatter, body_start.to_string())
}

/// Serializa el frontmatter (si existe) precediendo el contenido Markdown con `---`.
pub fn serialize_frontmatter_and_content(
    frontmatter: Option<&NoteFrontmatter>,
    body: &str,
) -> String {
    let Some(fm) = frontmatter else {
        return body.to_string();
    };

    if fm.is_empty() {
        return body.to_string();
    }

    match serde_yaml::to_string(fm) {
        Ok(yaml) => {
            let yaml_clean = yaml.trim_end();
            format!("---\n{}\n---\n{}", yaml_clean, body)
        }
        Err(err) => {
            log::error!("Error al serializar frontmatter YAML: {}", err);
            body.to_string()
        }
    }
}

/// Sincroniza el título en el frontmatter YAML de un contenido Markdown.
/// Si el contenido ya posee un bloque frontmatter, actualiza o añade el campo `title`.
/// Si no posee frontmatter, lo crea en la cabecera con el campo `title`.
pub fn sync_frontmatter_title(content: &str, title: &str) -> String {
    let (mut fm_opt, body) = parse_frontmatter_and_content(content);
    let mut fm = fm_opt.take().unwrap_or_default();
    fm.title = Some(title.to_string());
    serialize_frontmatter_and_content(Some(&fm), &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_basic() {
        let text = r#"---
title: Mi Nota
tags:
  - rust
  - testing
aliases:
  - NotaPrueba
type: concept
---
# Contenido de la nota
Este es el cuerpo principal."#;

        let (fm, content) = parse_frontmatter_and_content(text);
        assert!(fm.is_some());
        let fm = fm.unwrap();
        assert_eq!(fm.title.as_deref(), Some("Mi Nota"));
        assert_eq!(fm.tags, vec!["rust", "testing"]);
        assert_eq!(fm.aliases, vec!["NotaPrueba"]);
        assert_eq!(fm.node_type.as_deref(), Some("concept"));
        assert_eq!(
            content,
            "# Contenido de la nota\nEste es el cuerpo principal."
        );
    }

    #[test]
    fn test_parse_without_frontmatter() {
        let text = "# Solo Markdown\nSin frontmatter alguno";
        let (fm, content) = parse_frontmatter_and_content(text);
        assert!(fm.is_none());
        assert_eq!(content, text);
    }

    #[test]
    fn test_roundtrip_frontmatter() {
        let mut fm = NoteFrontmatter::default();
        fm.title = Some("Roundtrip Title".to_string());
        fm.tags = vec!["sync".to_string(), "okf".to_string()];

        let body = "# Cuerpo\nProbando roundtrip.";
        let serialized = serialize_frontmatter_and_content(Some(&fm), body);
        assert!(serialized.starts_with("---\n"));
        assert!(serialized.contains("title: Roundtrip Title"));

        let (deserialized_fm, deserialized_body) = parse_frontmatter_and_content(&serialized);
        assert_eq!(deserialized_fm, Some(fm));
        assert_eq!(deserialized_body, body);
    }

    #[test]
    fn test_sync_frontmatter_title() {
        let no_fm = "# Título Original\nTexto";
        let with_title = sync_frontmatter_title(no_fm, "Mi Archivo");
        let (fm, body) = parse_frontmatter_and_content(&with_title);
        assert_eq!(fm.unwrap().title.as_deref(), Some("Mi Archivo"));
        assert_eq!(body, no_fm);

        let existing_fm = "---\ntags:\n  - test\n---\n# Cuerpo";
        let updated = sync_frontmatter_title(existing_fm, "Nuevo Nombre");
        let (fm2, body2) = parse_frontmatter_and_content(&updated);
        let fm_data = fm2.unwrap();
        assert_eq!(fm_data.title.as_deref(), Some("Nuevo Nombre"));
        assert_eq!(fm_data.tags, vec!["test".to_string()]);
        assert_eq!(body2, "# Cuerpo");
    }
}
