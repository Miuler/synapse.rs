use std::path::PathBuf;

pub struct SearchInNoteQuery {
    pub vault_path: PathBuf,
    /// Una o varias notas donde buscar.
    pub relative_paths: Vec<String>,
    /// Texto a buscar; con `|` son alternativas (ver `split_alternatives`) salvo con `literal`.
    pub query: String,
    /// La consulta es un único texto literal aunque lleve `|`.
    pub literal: bool,
    /// Líneas de contexto a cada lado (máximo 10).
    pub context_lines: usize,
}

/// Línea que coincide, con su contexto numerado.
pub struct LineMatch {
    pub line_number: usize,
    pub matched_line: String,
    pub context: Vec<String>,
    /// Alternativas que aparecen en la línea (en el orden de la consulta).
    pub matched: Vec<String>,
}

/// Cuántas líneas contienen una alternativa y cuántas veces aparece en total.
pub struct AlternativeCount {
    pub alternative: String,
    pub lines: usize,
    pub occurrences: usize,
}

/// Coincidencias en una nota.
pub struct NoteMatches {
    pub matches: Vec<LineMatch>,
    /// Una entrada por alternativa, en el orden de la consulta (también las que no aparecen).
    pub counts: Vec<AlternativeCount>,
}

/// Resultado de una nota: sus coincidencias o por qué no se pudo leer.
pub struct NoteSearchOutcome {
    pub relative_path: String,
    pub outcome: Result<NoteMatches, String>,
}

pub struct SearchInNoteResult {
    /// Cómo se partió la consulta (una sola si no había alternativas).
    pub alternatives: Vec<String>,
    /// Una entrada por nota pedida, en el mismo orden.
    pub notes: Vec<NoteSearchOutcome>,
}
