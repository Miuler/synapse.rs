use crate::application::dto::search::index_vault_change_command::IndexVaultChangeCommand;
use crate::domain::search::indexed_document_repository::IndexedDocumentRepository;
use std::sync::Arc;

/// Mantiene el índice de búsqueda al día con los cambios de la bóveda. Solo encola: se invoca
/// desde hilos de escaneo y del vigilante de archivos, que no deben bloquearse.
pub struct IndexVaultChangeUseCase<R: IndexedDocumentRepository + ?Sized> {
    documents: Arc<R>,
}

impl<R: IndexedDocumentRepository + ?Sized> IndexVaultChangeUseCase<R> {
    pub fn new(documents: Arc<R>) -> Self {
        Self { documents }
    }

    pub fn execute(&self, command: IndexVaultChangeCommand) -> Result<(), String> {
        if !command.upserted.is_empty() {
            self.documents.schedule_upsert(command.upserted);
        }
        if !command.removed.is_empty() {
            self.documents.schedule_removal(command.removed);
        }
        Ok(())
    }
}
