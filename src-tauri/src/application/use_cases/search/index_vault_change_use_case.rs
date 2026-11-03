use crate::application::dto::search::index_vault_change_command::IndexVaultChangeCommand;
use crate::domain::search::indexed_document_repository::IndexedDocumentRepository;
use std::sync::Arc;

/// Mantiene el índice de búsqueda al día con los cambios de la bóveda. Solo encola: se invoca
/// desde hilos de escaneo y del vigilante de archivos, que no deben bloquearse.
pub struct IndexVaultChangeUseCase<R: IndexedDocumentRepository + ?Sized> {
    indexed_document_repository: Arc<R>,
}

impl<R: IndexedDocumentRepository + ?Sized> IndexVaultChangeUseCase<R> {
    pub fn new(indexed_document_repository: Arc<R>) -> Self {
        Self {
            indexed_document_repository,
        }
    }

    pub fn execute(&self, command: IndexVaultChangeCommand) -> Result<(), String> {
        if !command.upserted.is_empty() {
            self.indexed_document_repository
                .schedule_upsert(command.upserted);
        }
        if !command.removed.is_empty() {
            self.indexed_document_repository
                .schedule_removal(command.removed);
        }
        Ok(())
    }
}
