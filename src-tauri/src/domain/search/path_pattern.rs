use regex::{Regex, RegexBuilder};

/// Filtro de rutas de la bóveda: un glob (`*ddl*.sql`, `src/**/*.mmd`) o una expresión regular
/// (`entity|model|mapper|\.sql`). Sin `/`, el glob se compara con el nombre del archivo; con `/`,
/// con la ruta relativa entera. La regex se busca en cualquier parte de la ruta relativa.
#[derive(Debug, Clone)]
pub struct PathPattern {
    regex: Regex,
    name_only: bool,
}

impl PathPattern {
    pub fn glob(glob: &str, case_sensitive: bool) -> Result<Self, String> {
        let glob = glob.trim().trim_start_matches("./");
        let name_only = !glob.contains('/');
        let source = format!("^{}$", glob_to_regex(glob));
        Ok(Self {
            regex: build(&source, case_sensitive).map_err(|e| format!("Glob no válido «{glob}»: {e}"))?,
            name_only,
        })
    }

    pub fn regex(pattern: &str, case_sensitive: bool) -> Result<Self, String> {
        Ok(Self {
            regex: build(pattern, case_sensitive)
                .map_err(|e| format!("Expresión regular no válida «{pattern}»: {e}"))?,
            name_only: false,
        })
    }

    /// Si la ruta relativa (con `/`) cumple el patrón.
    pub fn matches(&self, relative_path: &str) -> bool {
        if self.name_only {
            let name = relative_path.rsplit('/').next().unwrap_or(relative_path);
            self.regex.is_match(name)
        } else {
            self.regex.is_match(relative_path)
        }
    }
}

fn build(source: &str, case_sensitive: bool) -> Result<Regex, regex::Error> {
    RegexBuilder::new(source)
        .case_insensitive(!case_sensitive)
        .size_limit(1 << 20)
        .build()
}

/// Traduce un glob a regex: `**/` cualquier número de carpetas, `**` cualquier cosa, `*` y `?`
/// dentro de un segmento, `{a,b}` alternativas y `[...]` clases de caracteres.
fn glob_to_regex(glob: &str) -> String {
    let chars: Vec<char> = glob.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut in_braces = false;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '*' if chars.get(i + 1) == Some(&'*') => {
                if chars.get(i + 2) == Some(&'/') {
                    out.push_str("(?:.*/)?");
                    i += 3;
                } else {
                    out.push_str(".*");
                    i += 2;
                }
                continue;
            }
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            '{' => {
                in_braces = true;
                out.push_str("(?:");
            }
            '}' if in_braces => {
                in_braces = false;
                out.push(')');
            }
            ',' if in_braces => out.push('|'),
            '[' => {
                if let Some(end) = chars[i + 1..].iter().position(|&ch| ch == ']') {
                    let class: String = chars[i + 1..i + 1 + end].iter().collect();
                    let class = class.strip_prefix('!').map(|r| format!("^{r}")).unwrap_or(class);
                    out.push('[');
                    out.push_str(&class.replace('\\', "\\\\"));
                    out.push(']');
                    i += end + 2;
                    continue;
                }
                out.push_str("\\[");
            }
            other => out.push_str(&regex::escape(&other.to_string())),
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_sin_barra_mira_solo_el_nombre() {
        let p = PathPattern::glob("*ddl*.sql", false).unwrap();
        assert!(p.matches("migrations/2026/020_schema_ach_operation_DDL.sql"));
        assert!(!p.matches("ddl/readme.md"));
    }

    #[test]
    fn glob_con_barra_mira_la_ruta_entera() {
        let p = PathPattern::glob("src/**/*.mmd", false).unwrap();
        assert!(p.matches("src/sequences/BS Financial Gateway.mmd"));
        assert!(p.matches("src/a.mmd"));
        assert!(!p.matches("docs/src/a.mmd"));
        assert!(!p.matches("src/sequences/a.d2"));
    }

    #[test]
    fn glob_con_llaves_y_clases() {
        let p = PathPattern::glob("*.{yaml,yml}", false).unwrap();
        assert!(p.matches("api/business-ach-operations.YAML"));
        assert!(p.matches("x.yml"));
        assert!(!p.matches("x.json"));
        let c = PathPattern::glob("QTI[13].mmd", true).unwrap();
        assert!(c.matches("QTI1.mmd") && c.matches("QTI3.mmd") && !c.matches("QTI2.mmd"));
        let n = PathPattern::glob("QTI[!1].mmd", true).unwrap();
        assert!(!n.matches("QTI1.mmd") && n.matches("QTI2.mmd"));
    }

    #[test]
    fn glob_escapa_los_caracteres_de_regex() {
        let p = PathPattern::glob("BS ACH (QTI1).mmd", false).unwrap();
        assert!(p.matches("src/BS ACH (QTI1).mmd"));
        assert!(!p.matches("src/BS ACH QTI1.mmd"));
    }

    #[test]
    fn regex_busca_en_toda_la_ruta() {
        let p = PathPattern::regex(r"entity|model|mapper|\.sql|repository", false).unwrap();
        assert!(p.matches("src/main/java/infrastructure/entity/OperationMasterEntity.java"));
        assert!(p.matches("db/001_init.SQL"));
        assert!(!p.matches("README.md"));
        assert!(PathPattern::regex("Model", true).unwrap().matches("a/Model.java"));
        assert!(!PathPattern::regex("Model", true).unwrap().matches("a/model.java"));
    }

    #[test]
    fn regex_no_valida_da_error_legible() {
        let e = PathPattern::regex("(abc", false).unwrap_err();
        assert!(e.contains("no válida"));
    }
}
