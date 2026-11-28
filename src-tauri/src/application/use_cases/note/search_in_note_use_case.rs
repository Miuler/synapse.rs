use crate::application::dto::note::search_in_note_query::{
    AlternativeCount, LineMatch, NoteMatches, NoteSearchOutcome, SearchInNoteQuery,
    SearchInNoteResult,
};
use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::note::note_repository::NoteRepository;
use crate::domain::search::alternatives::split_alternatives;

/// Busca uno o varios textos (alternativas `a|b`) dentro de una o varias notas, sin distinguir
/// mayúsculas, y devuelve cada línea que coincide con su contexto y los recuentos por alternativa.
pub struct SearchInNoteUseCase<N: NoteRepository> {
    note_repository: N,
}

impl<N: NoteRepository> SearchInNoteUseCase<N> {
    pub fn new(note_repository: N) -> Self {
        Self { note_repository }
    }

    pub fn execute(&self, query: SearchInNoteQuery) -> Result<SearchInNoteResult, String> {
        if query.relative_paths.is_empty() {
            return Err("Indica al menos una nota donde buscar".into());
        }
        let alternatives = if query.literal {
            vec![query.query.clone()]
        } else {
            split_alternatives(&query.query)
        };
        let lowered: Vec<String> = alternatives.iter().map(|a| a.to_lowercase()).collect();
        let context_lines = query.context_lines.min(10);
        let notes = query
            .relative_paths
            .iter()
            .map(|rel| NoteSearchOutcome {
                relative_path: rel.clone(),
                outcome: self.search_note(&query, rel, &alternatives, &lowered, context_lines),
            })
            .collect();
        Ok(SearchInNoteResult { alternatives, notes })
    }

    fn search_note(
        &self,
        query: &SearchInNoteQuery,
        rel: &str,
        alternatives: &[String],
        lowered: &[String],
        context_lines: usize,
    ) -> Result<NoteMatches, String> {
        let relative_path =
            NoteRelativePath::new(rel).map_err(|e| format!("Invalid path: {}", e))?;
        let note = self
            .note_repository
            .read_note(&query.vault_path, &relative_path)
            .map_err(|e| format!("Failed to read note: {}", e))?;

        let lines: Vec<&str> = note.content.lines().collect();
        let mut counts: Vec<AlternativeCount> = alternatives
            .iter()
            .map(|a| AlternativeCount { alternative: a.clone(), lines: 0, occurrences: 0 })
            .collect();
        let mut matches = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            let mut matched = Vec::new();
            for (k, alt) in lowered.iter().enumerate() {
                if !line_lower.contains(alt.as_str()) {
                    continue;
                }
                counts[k].lines += 1;
                // Una consulta vacía coincide con todas las líneas (como antes), una vez por línea
                counts[k].occurrences +=
                    if alt.is_empty() { 1 } else { line_lower.matches(alt.as_str()).count() };
                matched.push(alternatives[k].clone());
            }
            if matched.is_empty() {
                continue;
            }
            let start_idx = i.saturating_sub(context_lines);
            let end_idx = (i + context_lines + 1).min(lines.len());
            matches.push(LineMatch {
                line_number: i + 1,
                matched_line: line.to_string(),
                context: (start_idx..end_idx)
                    .map(|idx| format!("{}: {}", idx + 1, lines[idx]))
                    .collect(),
                matched,
            });
        }
        Ok(NoteMatches { matches, counts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
    use std::fs;

    fn vault() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("mapping.md"),
            "# Mapping\n| campo | tipo |\n| id_ach_operation | uuid |\nLímite BD: 20\nOtro límite bd y LÍMITE BD\nfin\n",
        )
        .unwrap();
        fs::write(dir.path().join("flujo.mmd"), "sequenceDiagram\n  MS->>TIEL: BCTITPE8\n").unwrap();
        dir
    }

    fn run(dir: &tempfile::TempDir, paths: &[&str], q: &str, literal: bool) -> SearchInNoteResult {
        SearchInNoteUseCase::new(NoteRepositoryImpl::new())
            .execute(SearchInNoteQuery {
                vault_path: dir.path().to_path_buf(),
                relative_paths: paths.iter().map(|p| p.to_string()).collect(),
                query: q.into(),
                literal,
                context_lines: 0,
            })
            .unwrap()
    }

    fn ok(r: &SearchInNoteResult, i: usize) -> &NoteMatches {
        r.notes[i].outcome.as_ref().unwrap()
    }

    #[test]
    fn alternativas_con_las_que_aparecen_en_cada_linea() {
        let dir = vault();
        let r = run(&dir, &["mapping.md"], "id_ach_operation|Límite BD|no almacena", false);
        assert_eq!(r.alternatives, ["id_ach_operation", "Límite BD", "no almacena"]);
        let m = ok(&r, 0);
        let lines: Vec<usize> = m.matches.iter().map(|l| l.line_number).collect();
        assert_eq!(lines, [3, 4, 5]);
        assert_eq!(m.matches[0].matched, ["id_ach_operation"]);
        assert_eq!(m.matches[1].matched, ["Límite BD"]);
    }

    #[test]
    fn recuentos_de_lineas_y_apariciones_sin_mayusculas() {
        let dir = vault();
        let r = run(&dir, &["mapping.md"], "límite bd|no almacena", false);
        let c = &ok(&r, 0).counts;
        assert_eq!((c[0].lines, c[0].occurrences), (2, 3), "«LÍMITE BD» también cuenta");
        assert_eq!((c[1].lines, c[1].occurrences), (0, 0), "las que no aparecen también se listan");
    }

    #[test]
    fn fila_de_tabla_markdown_es_literal() {
        let dir = vault();
        let r = run(&dir, &["mapping.md"], "| campo |", false);
        assert_eq!(r.alternatives, ["| campo |"]);
        assert_eq!(ok(&r, 0).matches.len(), 1);
        assert_eq!(ok(&r, 0).matches[0].line_number, 2);
        // Con literal, `a|b` tampoco se parte
        let r = run(&dir, &["mapping.md"], "campo | tipo", true);
        assert_eq!(r.alternatives, ["campo | tipo"]);
        assert_eq!(ok(&r, 0).matches.len(), 1);
    }

    #[test]
    fn varias_notas_y_una_que_no_existe() {
        let dir = vault();
        let r = run(&dir, &["mapping.md", "no-existe.md", "flujo.mmd"], "tiel|campo", false);
        assert_eq!(r.notes.len(), 3);
        assert_eq!(ok(&r, 0).matches.len(), 1);
        assert!(r.notes[1].outcome.is_err());
        assert_eq!(r.notes[1].relative_path, "no-existe.md");
        assert_eq!(ok(&r, 2).matches[0].matched, ["tiel"]);
    }

    #[test]
    fn consulta_sin_barras_se_comporta_como_antes() {
        let dir = vault();
        let r = run(&dir, &["flujo.mmd"], "ms->>", false);
        assert_eq!(r.alternatives, ["ms->>"]);
        assert_eq!(ok(&r, 0).matches.len(), 1);
    }
}
