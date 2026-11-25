use crate::application::dto::search::grep_vault_query::{
    GrepFile, GrepLine, GrepVaultQuery, GrepVaultResult,
};
use crate::application::use_cases::search::find_files_use_case::has_extension;
use crate::domain::search::path_pattern::PathPattern;
use crate::domain::vault::vault_item_repository::VaultItemRepository;
use rayon::prelude::*;
use regex::RegexBuilder;

/// Archivos mayores se omiten: no son texto que un agente vaya a leer línea a línea.
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
/// Las líneas largas (JSON minificado, XML de diagramas) se recortan alrededor de la coincidencia.
const MAX_LINE_CHARS: usize = 300;

/// Busca líneas por regex en los archivos de texto de la bóveda, como `rg`: código, SQL, YAML,
/// Mermaid, XML de ArchiMate… todos como texto plano. Los binarios (PDF, Word, imágenes) se omiten.
pub struct GrepVaultUseCase<V: VaultItemRepository> {
    vault_item_repository: V,
}

impl<V: VaultItemRepository> GrepVaultUseCase<V> {
    pub fn new(vault_item_repository: V) -> Self {
        Self { vault_item_repository }
    }

    pub fn execute(&self, query: GrepVaultQuery) -> Result<GrepVaultResult, String> {
        if query.pattern.is_empty() {
            return Err("El argumento 'pattern' no puede estar vacío".into());
        }
        let source = if query.literal { regex::escape(&query.pattern) } else { query.pattern.clone() };
        let re = RegexBuilder::new(&source)
            .case_insensitive(!query.case_sensitive)
            .size_limit(1 << 22)
            .build()
            .map_err(|e| format!("Expresión regular no válida «{}»: {e}", query.pattern))?;
        let glob = match query.glob.as_deref().filter(|g| !g.trim().is_empty()) {
            Some(g) => Some(PathPattern::glob(g, false)?),
            None => None,
        };
        let candidates: Vec<_> = self
            .vault_item_repository
            .list_vault_files(&query.vault_path, &query.folder)?
            .into_iter()
            .filter(|f| has_extension(&f.relative_path, &query.extensions))
            .filter(|f| glob.as_ref().is_none_or(|g| g.matches(&f.relative_path)))
            .collect();

        let per_file: Vec<Option<GrepFile>> = candidates
            .par_iter()
            .map(|f| {
                if f.size > MAX_FILE_BYTES {
                    return None;
                }
                let bytes = self
                    .vault_item_repository
                    .read_binary_item(&query.vault_path, &f.relative_path)
                    .ok()?;
                if looks_binary(&bytes) {
                    return None;
                }
                let text = String::from_utf8_lossy(&bytes);
                let lines: Vec<&str> = text.lines().collect();
                let mut matches = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    let Some(m) = re.find(line) else { continue };
                    if query.files_only {
                        matches.push(GrepLine { line: i + 1, text: String::new(), before: vec![], after: vec![] });
                        continue;
                    }
                    let from = i.saturating_sub(query.context_before);
                    let to = (i + 1 + query.context_after).min(lines.len());
                    matches.push(GrepLine {
                        line: i + 1,
                        text: clip(line, m.start(), m.end()),
                        before: lines[from..i].iter().map(|l| clip(l, 0, 0)).collect(),
                        after: lines[i + 1..to].iter().map(|l| clip(l, 0, 0)).collect(),
                    });
                }
                Some(GrepFile { path: f.relative_path.clone(), match_count: matches.len(), matches })
            })
            .collect();

        let files_skipped = per_file.iter().filter(|f| f.is_none()).count();
        let mut files: Vec<GrepFile> =
            per_file.into_iter().flatten().filter(|f| f.match_count > 0).collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let total_matches = files.iter().map(|f| f.match_count).sum();

        // Recorta las líneas devueltas a `max_matches`, conservando los recuentos completos
        let mut budget = query.max_matches;
        let mut truncated = false;
        for f in &mut files {
            if query.files_only {
                f.matches.clear();
                continue;
            }
            if f.matches.len() > budget {
                f.matches.truncate(budget);
                truncated = true;
            }
            budget -= f.matches.len();
        }
        if !query.files_only {
            let before = files.len();
            files.retain(|f| !f.matches.is_empty());
            truncated |= files.len() < before;
        }

        Ok(GrepVaultResult {
            files_searched: candidates.len() - files_skipped,
            files_skipped,
            total_matches,
            truncated,
            files,
        })
    }
}

/// Un byte nulo en los primeros 8 KB delata un binario (el mismo criterio que Git y ripgrep).
fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|&b| b == 0)
}

/// La línea sin espacios al final y, si es muy larga, recortada alrededor de `[start, end)`.
fn clip(line: &str, start: usize, end: usize) -> String {
    let line = line.trim_end();
    if line.chars().count() <= MAX_LINE_CHARS {
        return line.to_string();
    }
    let half = MAX_LINE_CHARS / 2;
    let mut from = start.saturating_sub(half);
    while !line.is_char_boundary(from) {
        from -= 1;
    }
    let mut to = (end + half).min(line.len()).max(from);
    while !line.is_char_boundary(to) {
        to += 1;
    }
    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    out.push_str(&line[from..to]);
    if to < line.len() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::search::find_files_use_case::test_support::MemoryFiles;
    use std::path::PathBuf;

    fn repo() -> MemoryFiles {
        MemoryFiles(vec![
            (
                "db/ddl.sql",
                b"-- tablas\nCREATE TABLE ach_operating_session (\n  id uuid\n);\nCREATE TABLE fee (\n  x int\n);\n",
            ),
            ("src/sequences/QTI1.mmd", b"sequenceDiagram\n  MS->>TIEL: BCTITPE8\n  MS->>DB: insert\n"),
            ("src/sequences/QTI3.mmd", b"sequenceDiagram\n  MS->>tiel: BCTITPE4\n"),
            ("img/a.png", b"\x89PNG\x00\x00TIEL"),
        ])
    }

    fn query(pattern: &str) -> GrepVaultQuery {
        GrepVaultQuery {
            vault_path: PathBuf::from("/v"),
            pattern: pattern.into(),
            literal: false,
            case_sensitive: false,
            folder: String::new(),
            glob: None,
            extensions: vec![],
            context_before: 0,
            context_after: 0,
            files_only: false,
            max_matches: 100,
        }
    }

    #[test]
    fn regex_con_alternativas_y_sin_mayusculas() {
        let r = GrepVaultUseCase::new(repo()).execute(query("TIEL|QTI1")).unwrap();
        assert_eq!(r.total_matches, 2);
        assert_eq!(r.files_skipped, 1, "el PNG es binario");
        assert_eq!(r.files[0].path, "src/sequences/QTI1.mmd");
        assert_eq!(r.files[0].matches[0].line, 2);
        assert_eq!(r.files[1].matches[0].text, "  MS->>tiel: BCTITPE4");
    }

    #[test]
    fn contexto_despues_como_rg_a() {
        let r = GrepVaultUseCase::new(repo())
            .execute(GrepVaultQuery {
                context_after: 2,
                extensions: vec!["sql".into()],
                ..query(r"CREATE TABLE.*(session|fee)")
            })
            .unwrap();
        assert_eq!(r.total_matches, 2);
        let first = &r.files[0].matches[0];
        assert_eq!(first.line, 2);
        assert_eq!(first.after, ["  id uuid", ");"]);
        assert!(first.before.is_empty());
    }

    #[test]
    fn solo_archivos_y_literal() {
        let r = GrepVaultUseCase::new(repo())
            .execute(GrepVaultQuery { files_only: true, literal: true, ..query("MS->>") })
            .unwrap();
        assert_eq!(r.files.len(), 2);
        assert!(r.files.iter().all(|f| f.matches.is_empty() && f.match_count > 0));
    }

    #[test]
    fn recorta_a_max_matches_y_lo_indica() {
        let r = GrepVaultUseCase::new(repo())
            .execute(GrepVaultQuery { max_matches: 1, ..query("MS") })
            .unwrap();
        assert_eq!(r.total_matches, 3);
        assert!(r.truncated);
        assert_eq!(r.files.len(), 1);
        assert_eq!(r.files[0].matches.len(), 1);
    }

    #[test]
    fn glob_y_carpeta_acotan() {
        let r = GrepVaultUseCase::new(repo())
            .execute(GrepVaultQuery { glob: Some("*3.mmd".into()), ..query("MS") })
            .unwrap();
        assert_eq!(r.total_matches, 1);
        let r = GrepVaultUseCase::new(repo())
            .execute(GrepVaultQuery { folder: "db".into(), ..query("MS") })
            .unwrap();
        assert_eq!(r.total_matches, 0);
    }

    #[test]
    fn linea_larga_recortada_alrededor_de_la_coincidencia() {
        let long = format!("{}AGUJA{}", "x".repeat(1000), "y".repeat(1000));
        let out = clip(&long, 1000, 1005);
        assert!(out.contains("AGUJA") && out.starts_with('…') && out.ends_with('…'));
        assert!(out.chars().count() < 320);
    }

    #[test]
    fn regex_no_valida() {
        assert!(GrepVaultUseCase::new(repo()).execute(query("(abc")).is_err());
    }
}
