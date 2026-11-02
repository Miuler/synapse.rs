//! Política de prioridad del Quick Open (Ctrl+O).
//!
//! Decide qué pesa más al buscar una nota: el nombre del archivo vale más que el título, el
//! título más que una etiqueta y una etiqueta más que la ruta; las notas abiertas hace poco
//! suben un poco. La calidad de la coincidencia difusa de cada campo (0..=1000) la calcula la
//! infraestructura; aquí solo se combina.

// Pesos por campo en por mil: la calidad de la coincidencia (0..=1000) se multiplica por el peso
// del campo donde ocurre, así una coincidencia en el nombre del archivo vale más que en la ruta.
pub const WEIGHT_NAME: u32 = 1000;
pub const WEIGHT_TITLE: u32 = 900;
pub const WEIGHT_TAG: u32 = 850;
pub const WEIGHT_PATH: u32 = 600;
// Coincidencias repartidas entre título y ruta (ej. "BCP" en el título y "TTIB" en la carpeta)
pub const WEIGHT_COMBINED: u32 = 550;

// Bonus aditivos (en la misma escala de 0..=1000)
pub const BONUS_EXACT_STEM: u32 = 150;
pub const BONUS_OPENED_LAST_DAY: u32 = 60;
pub const BONUS_OPENED_LAST_WEEK: u32 = 40;
pub const BONUS_OPENED_EVER: u32 = 15;

const DAY_NANOS: u128 = 24 * 60 * 60 * 1_000_000_000;

/// Campos de una nota que se comparan con la consulta.
pub struct QuickOpenCandidate<'a, T: AsRef<str>> {
    pub path: &'a str,
    pub title: &'a str,
    pub tags: &'a [T],
    pub last_opened_nanos: Option<u128>,
}

/// Combina la calidad de cada campo en el puntaje final de una nota para una consulta.
pub struct QuickOpenRanking {
    query_lower: String,
    now_nanos: u128,
}

impl QuickOpenRanking {
    pub fn new(query: &str, now_nanos: u128) -> Self {
        Self {
            query_lower: query.trim().to_lowercase(),
            now_nanos,
        }
    }

    /// Puntaje de la nota, o `None` si la consulta no coincide en ningún campo.
    ///
    /// `quality(texto, es_ruta)` devuelve la calidad de la coincidencia difusa en ese texto.
    pub fn score<T, F>(&self, candidate: &QuickOpenCandidate<'_, T>, mut quality: F) -> Option<u32>
    where
        T: AsRef<str>,
        F: FnMut(&str, bool) -> Option<u32>,
    {
        let path = candidate.path;
        let name = path.rsplit('/').next().unwrap_or(path);
        let title = candidate.title;

        let mut best = quality(name, false).map(|q| q * WEIGHT_NAME / 1000);
        best = best.max(quality(title, false).map(|q| q * WEIGHT_TITLE / 1000));
        for tag in candidate.tags {
            best = best.max(quality(tag.as_ref(), false).map(|q| q * WEIGHT_TAG / 1000));
        }
        best = best.max(quality(path, true).map(|q| q * WEIGHT_PATH / 1000));
        if best.is_none() {
            let combined = format!("{} {}", title, path);
            best = quality(&combined, true).map(|q| q * WEIGHT_COMBINED / 1000);
        }

        let mut score = best?;

        let name_lower = name.to_lowercase();
        let stem_lower = name_lower.rsplit_once('.').map_or(name_lower.as_str(), |(stem, _)| stem);
        if stem_lower == self.query_lower || name_lower == self.query_lower {
            score += BONUS_EXACT_STEM;
        }

        score += self.recency_bonus(candidate.last_opened_nanos);
        Some(score)
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

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u128 = 1_000 * DAY_NANOS;

    fn candidate<'a>(path: &'a str, opened: Option<u128>) -> QuickOpenCandidate<'a, &'a str> {
        QuickOpenCandidate { path, title: "", tags: &[], last_opened_nanos: opened }
    }

    #[test]
    fn name_outweighs_path_and_exact_stem_gets_bonus() {
        let ranking = QuickOpenRanking::new("pan", NOW);
        // Calidad fija de 1000 en cualquier campo que contenga "pan"
        let q = |h: &str, _: bool| h.contains("pan").then_some(1000);
        assert_eq!(ranking.score(&candidate("recetas/pan.md", None), q), Some(1000 + BONUS_EXACT_STEM));
        assert_eq!(ranking.score(&candidate("pan/receta.md", None), q), Some(WEIGHT_PATH));
    }

    #[test]
    fn recency_bonus_by_age() {
        let ranking = QuickOpenRanking::new("x", NOW);
        let q = |_: &str, _: bool| Some(0);
        assert_eq!(ranking.score(&candidate("a.md", Some(NOW - DAY_NANOS / 2)), q), Some(BONUS_OPENED_LAST_DAY));
        assert_eq!(ranking.score(&candidate("a.md", Some(NOW - 3 * DAY_NANOS)), q), Some(BONUS_OPENED_LAST_WEEK));
        assert_eq!(ranking.score(&candidate("a.md", Some(NOW - 30 * DAY_NANOS)), q), Some(BONUS_OPENED_EVER));
    }
}
