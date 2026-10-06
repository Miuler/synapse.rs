use crate::domain::models::full_text::{FullTextDocument, FullTextHit, FullTextQuery};
use std::collections::HashMap;

/// Puerto (Trait) del índice de búsqueda full-text dentro de la Capa de Dominio.
///
/// Las implementaciones concretas (p. ej. tantivy) viven en la capa de infraestructura.
pub trait FullTextIndex: Send + Sync {
    /// Inserta o reemplaza documentos (la clave es `FullTextDocument::path`).
    /// Los cambios no son visibles hasta llamar a [`FullTextIndex::commit`].
    fn upsert(&self, docs: Vec<FullTextDocument>) -> Result<(), String>;

    /// Elimina documentos por ruta relativa exacta.
    fn delete(&self, paths: &[String]) -> Result<(), String>;

    /// Elimina todos los documentos del índice.
    fn delete_all(&self) -> Result<(), String>;

    /// Persiste los cambios pendientes y los hace visibles para las búsquedas.
    fn commit(&self) -> Result<(), String>;

    /// Ejecuta una búsqueda. Devuelve los resultados y el número total de coincidencias.
    fn search(&self, query: &FullTextQuery) -> Result<(Vec<FullTextHit>, usize), String>;

    /// Devuelve `ruta relativa → mtime_nanos` de todos los documentos indexados.
    fn indexed_versions(&self) -> Result<HashMap<String, u64>, String>;

    /// Número de documentos visibles en el índice.
    fn num_docs(&self) -> u64;
}
