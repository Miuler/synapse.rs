use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use super::model::NoteMeta;
use crate::domain::service::quick_open_ranking::{QuickOpenCandidate, QuickOpenRanking};

/// Puntaje compuesto del Quick Open (Ctrl+O) construido sobre `nucleo_matcher`.
///
/// Cada campo de la nota (nombre de archivo, título, tags, ruta) se puntúa con el mismo patrón
/// difuso de nucleo. El puntaje crudo se normaliza contra el puntaje "ideal" (el patrón contra
/// sí mismo) para obtener una calidad comparable entre campos de distinta longitud, y se pondera
/// según el campo. El resultado es el mejor campo ponderado más bonus de recencia acotados, de
/// modo que una coincidencia dispersa reciente no tape a una coincidencia limpia nunca abierta.
pub struct QuickOpenScorer {
    pattern: Pattern,
    matcher: Matcher,
    path_matcher: Matcher,
    ideal: u32,
    ranking: QuickOpenRanking,
    buf: Vec<char>,
}

impl QuickOpenScorer {
    pub fn new(query: &str) -> Self {
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut buf = Vec::new();
        let ideal = pattern
            .score(Utf32Str::new(query, &mut buf), &mut matcher)
            .unwrap_or(0)
            .max(1);
        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        Self {
            pattern,
            matcher,
            path_matcher: Matcher::new(Config::DEFAULT.match_paths()),
            ideal,
            ranking: QuickOpenRanking::new(query, now_nanos),
            buf,
        }
    }

    /// Devuelve `None` si el patrón no coincide en ningún campo de la nota.
    pub fn score(&mut self, meta: &NoteMeta) -> Option<u32> {
        let candidate = QuickOpenCandidate {
            path: meta.path.as_str(),
            title: meta.title.as_str(),
            tags: &meta.tags,
            last_opened_nanos: meta.last_opened_nanos,
        };
        let Self { pattern, matcher, path_matcher, ideal, ranking, buf } = self;
        ranking.score(&candidate, |haystack, is_path| {
            field_quality(pattern, if is_path { path_matcher } else { matcher }, *ideal, buf, haystack)
        })
    }
}

/// Calidad de la coincidencia en el rango 0..=1000 (puede superar 1000 levemente si el campo
/// obtiene más bonus de límites de palabra que la propia consulta).
fn field_quality(pattern: &Pattern, matcher: &mut Matcher, ideal: u32, buf: &mut Vec<char>, haystack: &str) -> Option<u32> {
    let raw = pattern.score(Utf32Str::new(haystack, buf), matcher)?;
    Some(raw.saturating_mul(1000) / ideal)
}
