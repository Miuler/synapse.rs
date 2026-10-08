/// Servicio de dominio para análisis y normalización de enlaces estilo WikiLink (`[[...]]`)
/// y renderizado de enlaces en documentos Markdown.

/// Representa un token de WikiLink extraído de un texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikilinkToken {
    pub raw: String,
    pub target: String,
    pub alias: Option<String>,
    pub anchor: Option<String>,
    pub display_text: String,
    pub is_embed: bool,
    pub start: usize,
    pub end: usize,
}

/// Comprueba si una extensión corresponde a un tipo de archivo de imagen soportado.
pub fn is_image_extension(ext: &str) -> bool {
    let lower = ext.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" | "ico" | "avif" | "tiff" | "tif"
    )
}

/// Comprueba si una ruta o nombre de archivo corresponde a un archivo de imagen.
pub fn has_image_extension(path_or_name: &str) -> bool {
    let name = path_or_name.rsplit('/').next().unwrap_or(path_or_name);
    match name.rsplit_once('.') {
        Some((_, ext)) => is_image_extension(ext),
        None => false,
    }
}

/// Comprueba si un nombre o ruta de archivo tiene una extensión válida (ej. `txt`, `md`, `png`).
/// Ignora puntos que formen parte de numeraciones de títulos (ej. `1. Introducción`).
pub fn has_file_extension(path_or_name: &str) -> bool {
    let name = path_or_name.rsplit('/').next().unwrap_or(path_or_name);
    match name.rsplit_once('.') {
        Some((stem, ext)) => {
            !stem.is_empty()
                && !ext.is_empty()
                && !ext.contains(' ')
                && !ext.contains('/')
                && !ext.contains('\\')
                && ext.len() <= 10
        }
        None => false,
    }
}

/// Normaliza el objetivo de un enlace:
/// - Separa el ancla `#` si está presente.
/// - Si no tiene extensión, le añade automáticamente `.md`.
/// Retorna `(target_con_extension, anchor_opcional)`.
pub fn normalize_link_target(raw_target: &str) -> (String, Option<String>) {
    let clean = raw_target
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/');

    let (file_part, anchor_part) = match clean.split_once('#') {
        Some((f, a)) => (f.trim(), Some(a.trim().to_string())),
        None => (clean, None),
    };

    if file_part.is_empty() {
        return (String::new(), anchor_part);
    }

    let target_with_ext = if has_file_extension(file_part) {
        file_part.to_string()
    } else {
        format!("{}.md", file_part)
    };

    (target_with_ext, anchor_part)
}

/// Obtiene la carpeta contenedora normalizada (sin barra inicial ni final) de una ruta relativa.
pub fn get_parent_directory(path: &str) -> &str {
    let clean = path.trim().trim_start_matches("./").trim_start_matches('/');
    match clean.rfind('/') {
        Some(idx) => &clean[..idx],
        None => "",
    }
}

/// Puntuación de proximidad entre un archivo base y una ruta candidata.
/// Menor valor representa mayor preferencia / mayor cercanía.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProximityScore {
    /// 0: misma carpeta que el archivo base
    /// 1: subcarpeta de la carpeta del archivo base
    /// 2: carpeta ancestro / padre de la carpeta del archivo base
    /// 3: otra rama o carpeta diferente
    /// 4: sin archivo base de referencia
    pub tier: u8,
    /// Distancia en niveles jerárquicos de carpetas dentro del tier
    pub distance: usize,
    /// Longitud de la ruta (criterio de desempate para preferir rutas más concisas)
    pub path_len: usize,
}

/// Calcula la puntuación de cercanía entre un archivo candidato y un archivo base opcional.
/// Reglas de precedencia:
/// 1. Misma carpeta del archivo base (tier 0, dist 0)
/// 2. Subcarpetas del archivo base, priorizando las más cercanas (tier 1, menor profundidad relativa)
/// 3. Carpetas padre del archivo base hacia la raíz (tier 2, menor distancia hacia arriba)
/// 4. Otras ramas del árbol de directorios (tier 3, distancia total a través del ancestro común)
/// 5. Si no se pasa archivo base, se prefiere la menor profundidad hacia la raíz de la bóveda (tier 4)
pub fn calculate_proximity_score(candidate_path: &str, base_file: Option<&str>) -> ProximityScore {
    let cand_dir = get_parent_directory(candidate_path);

    let Some(base) = base_file else {
        let depth = if cand_dir.is_empty() { 0 } else { cand_dir.split('/').count() };
        return ProximityScore {
            tier: 4,
            distance: depth,
            path_len: candidate_path.len(),
        };
    };

    let base_dir = get_parent_directory(base);

    // 1. Misma carpeta
    if cand_dir == base_dir {
        return ProximityScore {
            tier: 0,
            distance: 0,
            path_len: candidate_path.len(),
        };
    }

    // 2. Subcarpeta de la carpeta actual
    if base_dir.is_empty() {
        if !cand_dir.is_empty() {
            let depth = cand_dir.split('/').count();
            return ProximityScore {
                tier: 1,
                distance: depth,
                path_len: candidate_path.len(),
            };
        }
    } else {
        let prefix = format!("{}/", base_dir);
        if let Some(rest) = cand_dir.strip_prefix(&prefix) {
            let depth = rest.split('/').count();
            return ProximityScore {
                tier: 1,
                distance: depth,
                path_len: candidate_path.len(),
            };
        }
    }

    // 3. Carpeta padre / ancestro de la carpeta actual
    if cand_dir.is_empty() {
        let depth = if base_dir.is_empty() { 0 } else { base_dir.split('/').count() };
        return ProximityScore {
            tier: 2,
            distance: depth,
            path_len: candidate_path.len(),
        };
    } else {
        let prefix = format!("{}/", cand_dir);
        if let Some(rest) = base_dir.strip_prefix(&prefix) {
            let depth = rest.split('/').count();
            return ProximityScore {
                tier: 2,
                distance: depth,
                path_len: candidate_path.len(),
            };
        }
    }

    // 4. Otra rama: distancia calculada mediante el ancestro común más cercano
    let base_parts: Vec<&str> = if base_dir.is_empty() { Vec::new() } else { base_dir.split('/').collect() };
    let cand_parts: Vec<&str> = if cand_dir.is_empty() { Vec::new() } else { cand_dir.split('/').collect() };

    let mut common = 0;
    while common < base_parts.len() && common < cand_parts.len() && base_parts[common] == cand_parts[common] {
        common += 1;
    }

    let up = base_parts.len() - common;
    let down = cand_parts.len() - common;

    ProximityScore {
        tier: 3,
        distance: up + down,
        path_len: candidate_path.len(),
    }
}

/// Extrae todos los WikiLinks (`[[...]]` o `![[...]`) de un contenido Markdown.
/// Omite texto dentro de bloques de código cercados o código en línea.
pub fn extract_wikilinks(content: &str) -> Vec<WikilinkToken> {
    let mut tokens = Vec::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    let mut in_fenced_block = false;
    let mut fence_char = b'`';
    let mut fence_len = 0;
    let mut in_inline_code = false;

    while i < len {
        // Manejo de bloques de código cercados (``` o ~~~)
        let is_line_start = i == 0 || bytes[i - 1] == b'\n';
        if is_line_start && !in_inline_code {
            if (bytes[i] == b'`' || bytes[i] == b'~') && i + 2 < len && bytes[i + 1] == bytes[i] && bytes[i + 2] == bytes[i] {
                let current_fence = bytes[i];
                let mut flen = 0;
                while i + flen < len && bytes[i + flen] == current_fence {
                    flen += 1;
                }
                if flen >= 3 {
                    if in_fenced_block && current_fence == fence_char && flen >= fence_len {
                        in_fenced_block = false;
                        i += flen;
                        continue;
                    } else if !in_fenced_block {
                        in_fenced_block = true;
                        fence_char = current_fence;
                        fence_len = flen;
                        i += flen;
                        continue;
                    }
                }
            }
        }

        if in_fenced_block {
            i += 1;
            continue;
        }

        // Manejo de código en línea (`...`)
        if bytes[i] == b'`' {
            in_inline_code = !in_inline_code;
            i += 1;
            continue;
        }

        if in_inline_code {
            i += 1;
            continue;
        }

        // Detección de WikiLink `[[...]]` o embed `![[...]]`
        if i + 1 < len && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let is_embed = i > 0 && bytes[i - 1] == b'!';
            let token_start = if is_embed { i - 1 } else { i };

            let inner_start = i + 2;
            let mut close_pos = None;

            let mut j = inner_start;
            while j + 1 < len {
                if bytes[j] == b'\n' {
                    break;
                }
                if bytes[j] == b']' && bytes[j + 1] == b']' {
                    close_pos = Some(j);
                    break;
                }
                j += 1;
            }

            if let Some(inner_end) = close_pos {
                let inner = &content[inner_start..inner_end];
                let raw = &content[token_start..inner_end + 2];

                if !inner.trim().is_empty() {
                    let (target_part, alias_part) = match inner.split_once('|') {
                        Some((t, a)) => (t.trim(), Some(a.trim())),
                        None => (inner.trim(), None),
                    };

                    let (file_target, anchor) = match target_part.split_once('#') {
                        Some((f, a)) => (f.trim(), Some(a.trim().to_string())),
                        None => (target_part, None),
                    };

                    let display_text = match alias_part {
                        Some(a) if !a.is_empty() => a.to_string(),
                        _ => {
                            if !target_part.is_empty() {
                                target_part.to_string()
                            } else if let Some(ref a) = anchor {
                                format!("#{}", a)
                            } else {
                                inner.trim().to_string()
                            }
                        }
                    };

                    tokens.push(WikilinkToken {
                        raw: raw.to_string(),
                        target: file_target.to_string(),
                        alias: alias_part.map(|s| s.to_string()),
                        anchor,
                        display_text,
                        is_embed,
                        start: token_start,
                        end: inner_end + 2,
                    });
                }

                i = inner_end + 2;
                continue;
            }
        }

        i += 1;
    }

    tokens
}

/// Transforma el contenido Markdown sustituyendo las apariciones de WikiLinks:
/// - Si el objetivo es una imagen (ej. `[[FILE.png]]` o `![[FILE.png]]`), se transforma en
///   `![display_text](<resolved_path>)` para que sea renderizada como `<img>`.
/// - Si no es imagen (ej. `[[FILE.md]]` o `[[FILE.txt]]`), se transforma en `[display_text](<resolved_path>)`
///   para que sea renderizada como un enlace `<a>` navegable.
///
/// Si el archivo no tiene extensión, se autocompleta con `.md` al buscar en el DashMap y al generar el fallback.
pub fn transform_markdown_wikilinks<F>(content: &str, resolver: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let tokens = extract_wikilinks(content);
    if tokens.is_empty() {
        return content.to_string();
    }

    let mut result = String::with_capacity(content.len() + tokens.len() * 16);
    let mut last_idx = 0;

    for token in tokens {
        result.push_str(&content[last_idx..token.start]);

        let is_image = has_image_extension(&token.target);

        let resolved_path = if token.target.is_empty() {
            // Enlace interno sólo con ancla (#sección)
            token.anchor.as_ref().map(|a| format!("#{}", a)).unwrap_or_default()
        } else {
            let (target_with_ext, _) = normalize_link_target(&token.target);

            // Intentar resolver primero con el target original, luego con la extensión añadida
            let path_found = resolver(&token.target)
                .or_else(|| resolver(&target_with_ext));

            match path_found {
                Some(p) => match (&token.anchor, p.contains('#')) {
                    (Some(a), false) => format!("{}#{}", p, a),
                    _ => p,
                },
                None => match &token.anchor {
                    Some(a) => format!("{}#{}", target_with_ext, a),
                    None => target_with_ext,
                },
            }
        };

        // Si es un archivo de imagen (o un embed explícito), se formatea como imagen Markdown `![...]`
        // lo que produce una etiqueta `<img>` insertada en el renderizado.
        // En caso contrario, se formatea como enlace `[...]` que produce `<a>`.
        let md_link = if is_image || token.is_embed {
            if resolved_path.contains(' ') {
                format!("![{}](<{}>)", token.display_text, resolved_path)
            } else {
                format!("![{}]({})", token.display_text, resolved_path)
            }
        } else if resolved_path.contains(' ') {
            format!("[{}](<{}>)", token.display_text, resolved_path)
        } else {
            format!("[{}]({})", token.display_text, resolved_path)
        };

        result.push_str(&md_link);
        last_idx = token.end;
    }

    result.push_str(&content[last_idx..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_extension() {
        assert!(is_image_extension("png"));
        assert!(is_image_extension("PNG"));
        assert!(is_image_extension("webp"));
        assert!(is_image_extension("jpg"));
        assert!(is_image_extension("jpeg"));
        assert!(is_image_extension("svg"));
        assert!(is_image_extension("gif"));
        assert!(is_image_extension("bmp"));
        assert!(is_image_extension("ico"));
        assert!(is_image_extension("avif"));
        assert!(is_image_extension("tiff"));
        assert!(is_image_extension("tif"));
        assert!(!is_image_extension("md"));
        assert!(!is_image_extension("txt"));
        assert!(!is_image_extension("pdf"));
    }

    #[test]
    fn test_has_image_extension() {
        assert!(has_image_extension("FILE.png"));
        assert!(has_image_extension("sub/carpeta/foto.jpg"));
        assert!(has_image_extension("diagram.SVG"));
        assert!(!has_image_extension("FILE.md"));
        assert!(!has_image_extension("FILE.txt"));
        assert!(!has_image_extension("FILE"));
    }

    #[test]
    fn test_has_file_extension() {
        assert!(has_file_extension("archivo.txt"));
        assert!(has_file_extension("sub/carpeta/nota.md"));
        assert!(has_file_extension("foto.png"));
        assert!(has_file_extension("doc.tar.gz"));
        assert!(!has_file_extension("archivo"));
        assert!(!has_file_extension("1. Introducción"));
        assert!(!has_file_extension("sub/carpeta/sin_extension"));
    }

    #[test]
    fn test_normalize_link_target() {
        assert_eq!(
            normalize_link_target("NOMBRE ARCHIVO.txt"),
            ("NOMBRE ARCHIVO.txt".to_string(), None)
        );
        assert_eq!(
            normalize_link_target("NOMBRE ARCHIVO"),
            ("NOMBRE ARCHIVO.md".to_string(), None)
        );
        assert_eq!(
            normalize_link_target("carpeta/NOMBRE ARCHIVO"),
            ("carpeta/NOMBRE ARCHIVO.md".to_string(), None)
        );
        assert_eq!(
            normalize_link_target("NOMBRE ARCHIVO.txt#seccion 1"),
            (
                "NOMBRE ARCHIVO.txt".to_string(),
                Some("seccion 1".to_string())
            )
        );
        assert_eq!(
            normalize_link_target("NOMBRE ARCHIVO#seccion 1"),
            (
                "NOMBRE ARCHIVO.md".to_string(),
                Some("seccion 1".to_string())
            )
        );
    }

    #[test]
    fn test_extract_wikilinks_basic_and_aliases() {
        let text = "Ver [[NOMBRE ARCHIVO.txt]] y también [[Guía Rápida|Ir a la guía]].";
        let tokens = extract_wikilinks(text);
        assert_eq!(tokens.len(), 2);

        assert_eq!(tokens[0].target, "NOMBRE ARCHIVO.txt");
        assert_eq!(tokens[0].display_text, "NOMBRE ARCHIVO.txt");
        assert_eq!(tokens[0].alias, None);
        assert!(!tokens[0].is_embed);

        assert_eq!(tokens[1].target, "Guía Rápida");
        assert_eq!(tokens[1].display_text, "Ir a la guía");
        assert_eq!(tokens[1].alias, Some("Ir a la guía".to_string()));
        assert!(!tokens[1].is_embed);
    }

    #[test]
    fn test_extract_wikilinks_captures_embeds_and_ignores_code() {
        let text = r#"
Transclusión: ![[imagen.png]]
Código en línea: `[[ignorar_esto]]`
Bloque cercado:
```
[[ignorar_en_bloque]]
```
Enlace real: [[Mi Nota]]
"#;
        let tokens = extract_wikilinks(text);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].target, "imagen.png");
        assert!(tokens[0].is_embed);
        assert_eq!(tokens[1].target, "Mi Nota");
        assert!(!tokens[1].is_embed);
    }

    #[test]
    fn test_transform_markdown_wikilinks_images_vs_links() {
        let content = "Nota [[FILE.md]] junto a imagen [[FILE.png]] y otra [[sub/banner.webp|300]].";

        let fake_dashmap = |query: &str| -> Option<String> {
            match query {
                "FILE.md" => Some("docs/FILE.md".to_string()),
                "FILE.png" => Some("assets/img/FILE.png".to_string()),
                "sub/banner.webp" | "banner.webp" => Some("assets/sub/banner.webp".to_string()),
                _ => None,
            }
        };

        let transformed = transform_markdown_wikilinks(content, fake_dashmap);
        assert_eq!(
            transformed,
            "Nota [FILE.md](docs/FILE.md) junto a imagen ![FILE.png](assets/img/FILE.png) y otra ![300](assets/sub/banner.webp)."
        );
    }

    #[test]
    fn test_proximity_score_preference() {
        let base_file = "Trabajo/Kyndryl/BCP/notas.md";

        let same_folder = "Trabajo/Kyndryl/BCP/foto.png";
        let subfolder_close = "Trabajo/Kyndryl/BCP/adjuntos/foto.png";
        let subfolder_deep = "Trabajo/Kyndryl/BCP/adjuntos/deep/foto.png";
        let parent_folder = "Trabajo/Kyndryl/foto.png";
        let grandparent_folder = "Trabajo/foto.png";
        let other_branch = "Personal/foto.png";

        let score_same = calculate_proximity_score(same_folder, Some(base_file));
        let score_sub_close = calculate_proximity_score(subfolder_close, Some(base_file));
        let score_sub_deep = calculate_proximity_score(subfolder_deep, Some(base_file));
        let score_parent = calculate_proximity_score(parent_folder, Some(base_file));
        let score_grandparent = calculate_proximity_score(grandparent_folder, Some(base_file));
        let score_other = calculate_proximity_score(other_branch, Some(base_file));

        // 1. Same folder (tier 0) beats subfolder (tier 1)
        assert!(score_same < score_sub_close);

        // 2. Closest subfolder (tier 1, dist 1) beats deeper subfolder (tier 1, dist 2)
        assert!(score_sub_close < score_sub_deep);

        // 3. Subfolders (tier 1) beat parent folder (tier 2)
        assert!(score_sub_close < score_parent);
        assert!(score_sub_deep < score_parent);

        // 4. Closer parent beats farther grandparent
        assert!(score_parent < score_grandparent);

        // 5. Parent folders (tier 2) beat other branch (tier 3)
        assert!(score_grandparent < score_other);

        // Complete ordering check
        let mut list = vec![
            other_branch,
            grandparent_folder,
            subfolder_deep,
            parent_folder,
            subfolder_close,
            same_folder,
        ];
        list.sort_by_key(|cand| calculate_proximity_score(cand, Some(base_file)));

        assert_eq!(
            list,
            vec![
                same_folder,
                subfolder_close,
                subfolder_deep,
                parent_folder,
                grandparent_folder,
                other_branch,
            ]
        );
    }
}
