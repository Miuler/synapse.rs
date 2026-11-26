use crate::domain::search::full_text::{FullTextDocument, FullTextHit, FullTextQuery, MatchedLine};
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

    /// Líneas de `text` que coinciden con la consulta, analizadas igual que en la búsqueda
    /// (o literales en el modo exacto), con su número de línea real y hasta
    /// `query.lines.max_per_doc`; devuelve también cuántas coinciden en total. Sirve para dar
    /// números de línea del archivo cuando el índice guarda un texto procesado (Markdown).
    fn match_lines(&self, query: &FullTextQuery, text: &str) -> (Vec<MatchedLine>, usize);

    /// Como `match_lines`, sobre el texto que el índice guarda del documento `path` (el texto
    /// extraído de un PDF, una planilla o un modelo), con la ubicación propia de su formato.
    fn match_document_lines(&self, query: &FullTextQuery, path: &str) -> (Vec<MatchedLine>, usize);
}

/// Permite pasar el repositorio por referencia a un caso de uso sin moverlo ni clonarlo.
impl<T: FullTextIndex + ?Sized> FullTextIndex for &T {
    fn match_lines(&self, query: &FullTextQuery, text: &str) -> (Vec<MatchedLine>, usize) {
        (**self).match_lines(query, text)
    }

    fn match_document_lines(&self, query: &FullTextQuery, path: &str) -> (Vec<MatchedLine>, usize) {
        (**self).match_document_lines(query, path)
    }

    fn upsert(&self, docs: Vec<FullTextDocument>) -> Result<(), String> {
        (**self).upsert(docs)
    }

    fn delete(&self, paths: &[String]) -> Result<(), String> {
        (**self).delete(paths)
    }

    fn delete_all(&self) -> Result<(), String> {
        (**self).delete_all()
    }

    fn commit(&self) -> Result<(), String> {
        (**self).commit()
    }

    fn search(&self, query: &FullTextQuery) -> Result<(Vec<FullTextHit>, usize), String> {
        (**self).search(query)
    }

    fn indexed_versions(&self) -> Result<HashMap<String, u64>, String> {
        (**self).indexed_versions()
    }

    fn num_docs(&self) -> u64 {
        (**self).num_docs()
    }
}
