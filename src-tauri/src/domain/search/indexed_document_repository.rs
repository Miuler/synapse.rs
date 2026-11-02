/// Documentos del índice de búsqueda de texto completo (los construimos nosotros a partir de la
/// bóveda). Las operaciones se encolan: no bloquean a quien las pide.
pub trait IndexedDocumentRepository: Send + Sync {
    /// Programa la (re)indexación de archivos añadidos o modificados.
    fn schedule_upsert(&self, paths: Vec<String>);
    /// Programa la eliminación del índice de archivos o carpetas borrados.
    fn schedule_removal(&self, paths: Vec<String>);
}
