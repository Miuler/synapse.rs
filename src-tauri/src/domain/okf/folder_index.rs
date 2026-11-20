//! Índice de carpeta `index.md` según Open Knowledge Format (OKF) v0.3, generado por Synapse.
//!
//! - Sin frontmatter, salvo el de la raíz de la bóveda: `okf_version`, y se conservan `title`,
//!   `description`, `license` y las claves `x-…` que ya tuviera (la especificación lo exige).
//! - Secciones con `#` y entradas `* [Título](enlace) - descripción`; las subcarpetas con barra final.
//! - «Conceptos» son los `.md` con `type` en su frontmatter; el resto de archivos van en «Archivos».
//! - Salida determinista (orden fijo, sin fechas): si no cambia nada, el archivo queda idéntico.
use serde_yaml::Value;

/// Marca que identifica un `index.md` generado por Synapse (uno sin ella es del usuario)
pub const GENERATED_MARKER: &str = "<!-- okf-index: generado por Synapse a partir de la carpeta; no editar (se regenera solo) -->";
pub const OKF_VERSION: &str = "0.3";
pub const INDEX_FILE: &str = "index.md";
pub const LOG_FILE: &str = "log.md";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    /// Nombre del archivo o de la carpeta dentro de la carpeta indexada
    pub name: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderIndex {
    pub subfolders: Vec<IndexEntry>,
    pub concepts: Vec<IndexEntry>,
    pub files: Vec<IndexEntry>,
}

/// ¿Lo generó Synapse? (busca la marca al principio, después del frontmatter si lo hay)
pub fn is_generated(content: &str) -> bool {
    content.lines().take(20).any(|l| l.trim() == GENERATED_MARKER)
}

/// Enlace relativo con los caracteres especiales codificados (`%20` para espacios), como pide OKF
pub fn encode_link(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' => out.push(ch),
            c => {
                let mut buf = [0u8; 4];
                for b in c.encode_utf8(&mut buf).bytes() {
                    out.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    out
}

/// Texto de una entrada en una sola línea (sin saltos ni corchetes que rompan el enlace)
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").replace(['[', ']'], "")
}

impl FolderIndex {
    fn sort(&mut self) {
        let key = |e: &IndexEntry| (e.title.to_lowercase(), e.name.clone());
        self.subfolders.sort_by_key(key);
        self.concepts.sort_by_key(key);
        self.files.sort_by_key(key);
    }

    /// `index.md` de la carpeta. `root_frontmatter`: el del `index.md` de la raíz que ya existía
    /// (solo para la raíz de la bóveda).
    pub fn render(&self, is_root: bool, root_frontmatter: Option<&str>) -> String {
        let mut me = self.clone();
        me.sort();
        let mut out = String::new();
        if is_root {
            out.push_str(&root_frontmatter_yaml(root_frontmatter));
        }
        out.push_str(GENERATED_MARKER);
        out.push('\n');
        let section = |out: &mut String, title: &str, entries: &[IndexEntry], dir: bool| {
            if entries.is_empty() {
                return;
            }
            out.push_str(&format!("\n# {title}\n\n"));
            for e in entries {
                let link = if dir { format!("{}/", encode_link(&e.name)) } else { encode_link(&e.name) };
                match e.description.as_deref().map(one_line).filter(|d| !d.is_empty()) {
                    Some(d) => out.push_str(&format!("* [{}]({}) - {}\n", one_line(&e.title), link, d)),
                    None => out.push_str(&format!("* [{}]({})\n", one_line(&e.title), link)),
                }
            }
        };
        section(&mut out, "Carpetas", &me.subfolders, true);
        section(&mut out, "Conceptos", &me.concepts, false);
        section(&mut out, "Archivos", &me.files, false);
        out
    }
}

/// Frontmatter de la raíz: `okf_version` y lo que se conserva del anterior (`title`, `description`,
/// `license`, `x-…`), en orden fijo
fn root_frontmatter_yaml(previous: Option<&str>) -> String {
    let mut map = serde_yaml::Mapping::new();
    map.insert(Value::from("okf_version"), Value::from(OKF_VERSION));
    if let Some(Value::Mapping(prev)) = previous.and_then(|p| serde_yaml::from_str::<Value>(p).ok()) {
        let mut keys: Vec<(String, Value)> = prev
            .into_iter()
            .filter_map(|(k, v)| k.as_str().map(|k| (k.to_string(), v)))
            .filter(|(k, _)| matches!(k.as_str(), "title" | "description" | "license") || k.starts_with("x-"))
            .collect();
        keys.sort_by(|a, b| a.0.cmp(&b.0));
        for (k, v) in keys {
            map.insert(Value::from(k), v);
        }
    }
    let yaml = serde_yaml::to_string(&map).unwrap_or_default();
    // okf_version entre comillas (la especificación lo recomienda para valores que parecen números)
    let yaml = yaml.replacen(&format!("okf_version: '{OKF_VERSION}'"), &format!("okf_version: \"{OKF_VERSION}\""), 1);
    format!("---\n{yaml}---\n")
}

/// Frontmatter (texto YAML sin los `---`) de un contenido, si lo tiene
pub fn frontmatter_block(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    Some(&rest[..end + 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str, title: &str, desc: Option<&str>) -> IndexEntry {
        IndexEntry { name: name.into(), title: title.into(), description: desc.map(String::from) }
    }

    #[test]
    fn carpeta_con_secciones_ordenadas_y_enlaces_codificados() {
        let idx = FolderIndex {
            subfolders: vec![e("Teams - AS41", "Teams - AS41", Some("2 conceptos, 1 archivo"))],
            concepts: vec![e("plan.md", "Plan", Some("Plan del\ntrimestre")), e("b.md", "Alfa", None)],
            files: vec![e("BCP Glosario.docx", "BCP Glosario.docx", None)],
        };
        let out = idx.render(false, None);
        assert_eq!(
            out,
            format!(
                "{GENERATED_MARKER}\n\n# Carpetas\n\n* [Teams - AS41](Teams%20-%20AS41/) - 2 conceptos, 1 archivo\n\n# Conceptos\n\n* [Alfa](b.md)\n* [Plan](plan.md) - Plan del trimestre\n\n# Archivos\n\n* [BCP Glosario.docx](BCP%20Glosario.docx)\n"
            )
        );
        assert!(is_generated(&out));
        assert!(!is_generated("# Mi índice\n"));
        // Determinista
        assert_eq!(out, idx.render(false, None));
    }

    #[test]
    fn la_raiz_lleva_okf_version_y_conserva_titulo_y_extensiones() {
        let prev = "title: Mis notas\nokf_version: \"0.2\"\nx-autor: yo\notra: no\n";
        let out = FolderIndex::default().render(true, Some(prev));
        assert!(out.starts_with("---\nokf_version: \"0.3\"\n"), "{out}");
        assert!(out.contains("title: Mis notas"));
        assert!(out.contains("x-autor: yo"));
        assert!(!out.contains("otra:"));
        assert!(is_generated(&out));
        assert_eq!(frontmatter_block(&out).map(|f| f.contains("okf_version")), Some(true));
    }
}
