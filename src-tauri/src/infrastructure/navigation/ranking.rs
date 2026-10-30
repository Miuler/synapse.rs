use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use super::model::NoteMeta;

// Pesos por campo en por mil: la calidad de la coincidencia (0..=1000) se multiplica por el peso
// del campo donde ocurre, así una coincidencia en el nombre del archivo vale más que en la ruta.
const WEIGHT_NAME: u32 = 1000;
const WEIGHT_TITLE: u32 = 900;
const WEIGHT_TAG: u32 = 850;
const WEIGHT_PATH: u32 = 600;
// Coincidencias repartidas entre título y ruta (ej. "BCP" en el título y "TTIB" en la carpeta)
const WEIGHT_COMBINED: u32 = 550;

// Bonus aditivos (en la misma escala de 0..=1000)
const BONUS_EXACT_STEM: u32 = 150;
const BONUS_OPENED_LAST_DAY: u32 = 60;
const BONUS_OPENED_LAST_WEEK: u32 = 40;
const BONUS_OPENED_EVER: u32 = 15;

const DAY_NANOS: u128 = 24 * 60 * 60 * 1_000_000_000;

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
    query_lower: String,
    now_nanos: u128,
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
            query_lower: query.trim().to_lowercase(),
            now_nanos,
            buf,
        }
    }

    /// Devuelve `None` si el patrón no coincide en ningún campo de la nota.
    pub fn score(&mut self, meta: &NoteMeta) -> Option<u32> {
        let path = meta.path.as_str();
        let name = path.rsplit('/').next().unwrap_or(path);
        let title = meta.title.as_str();

        let mut best = self.field_quality(name, false).map(|q| q * WEIGHT_NAME / 1000);
        best = best.max(self.field_quality(title, false).map(|q| q * WEIGHT_TITLE / 1000));
        for tag in &meta.tags {
            best = best.max(self.field_quality(tag.as_str(), false).map(|q| q * WEIGHT_TAG / 1000));
        }
        best = best.max(self.field_quality(path, true).map(|q| q * WEIGHT_PATH / 1000));
        if best.is_none() {
            let combined = format!("{} {}", title, path);
            best = self.field_quality(&combined, true).map(|q| q * WEIGHT_COMBINED / 1000);
        }

        let mut score = best?;

        let name_lower = name.to_lowercase();
        let stem_lower = name_lower.rsplit_once('.').map_or(name_lower.as_str(), |(stem, _)| stem);
        if stem_lower == self.query_lower || name_lower == self.query_lower {
            score += BONUS_EXACT_STEM;
        }

        score += self.recency_bonus(meta.last_opened_nanos);
        Some(score)
    }

    /// Calidad de la coincidencia en el rango 0..=1000 (puede superar 1000 levemente si el campo
    /// obtiene más bonus de límites de palabra que la propia consulta).
    fn field_quality(&mut self, haystack: &str, is_path: bool) -> Option<u32> {
        let matcher = if is_path { &mut self.path_matcher } else { &mut self.matcher };
        let raw = self
            .pattern
            .score(Utf32Str::new(haystack, &mut self.buf), matcher)?;
        Some(raw.saturating_mul(1000) / self.ideal)
    }

    fn recency_bonus(&self, last_opened_nanos: Option<u128>) -> u32 {
        let Some(opened) = last_opened_nanos else {
            return 0;
        };
        let age = self.now_nanos.saturating_sub(opened);
        if age <= DAY_NANOS {
            BONUS_OPENED_LAST_DAY
        } else if age <= 7 * DAY_NANOS {
            BONUS_OPENED_LAST_WEEK
        } else {
            BONUS_OPENED_EVER
        }
    }
}
