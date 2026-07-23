use crate::domain::models::note::Note;
use serde::{Deserialize, Serialize};

/// Estructura de valor que representa el resultado de una coincidencia de búsqueda difusa (Fuzzy Search).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Texto o título donde se encontró la coincidencia.
    pub text: String,

    /// Puntuación numérica asignada por el motor de búsqueda difusa (`nucleo-matcher`).
    /// A mayor valor, mayor relevancia de la coincidencia.
    pub score: u32,

    /// Lista de índices de los caracteres del texto que coincidieron exactamente con la consulta.
    /// Utilizado por el frontend para resaltar visualmente las letras coincidentes.
    pub match_indices: Vec<u32>,

    /// Ruta relativa de la nota asociada a la coincidencia (si aplica).
    pub note_path: Option<String>,
}

/// Puerto (Trait) del servicio de búsqueda difusa dentro de la Capa de Dominio.
///
/// Define las operaciones de búsqueda aceleradas en memoria tanto para cadenas cortas (comandos, títulos)
/// como para búsqueda profunda en el texto completo de notas.
///
/// Nota sobre `rustdoc`: Las implementaciones (`impl SearchService for Struct`)
/// heredan automáticamente esta documentación en la generación de `cargo doc`.
pub trait SearchService: Send + Sync {
    /// Realiza una búsqueda difusa rápida en una lista de cadenas simples (ej. comandos o nombres de archivo).
    ///
    /// # Parámetros
    /// * `query`: La cadena de texto o patrón buscado introducido por el usuario.
    /// * `items`: Un slice de cadenas de texto (`&[String]`) sobre las cuales realizar el filtrado.
    ///
    /// # Retorno
    /// * `Vec<SearchResult>`: Lista de resultados ordenada de mayor a menor relevancia por `score`.
    fn search_items(&self, query: &str, items: &[String]) -> Vec<SearchResult>;

    /// Realiza una búsqueda difusa profunda tanto en los títulos como en el contenido completo de un conjunto de notas.
    ///
    /// # Parámetros
    /// * `query`: La cadena de texto o patrón de búsqueda introducido por el usuario.
    /// * `notes`: Un slice de entidades de dominio `Note` (`&[Note]`) sobre las cuales buscar.
    ///
    /// # Retorno
    /// * `Vec<SearchResult>`: Lista de coincidencias ordenadas por relevancia por `score`.
    fn search_notes(&self, query: &str, notes: &[Note]) -> Vec<SearchResult>;
}
