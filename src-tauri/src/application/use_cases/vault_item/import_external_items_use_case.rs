use crate::application::dto::vault_item::import_external_items_command::ImportExternalItemsCommand;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Pega en una carpeta de la bóveda archivos o carpetas de fuera (lo copiado en otra instancia o
/// en el explorador de archivos del sistema) y devuelve las rutas creadas.
#[derive(Clone)]
pub struct ImportExternalItemsUseCase<A: VaultItemRepository> {
    vault_item_repository: A,
}

impl<A: VaultItemRepository> ImportExternalItemsUseCase<A> {
    pub fn new(vault_item_repository: A) -> Self {
        Self {
            vault_item_repository,
        }
    }

    pub fn execute(&self, command: ImportExternalItemsCommand) -> Result<Vec<String>, String> {
        let mut created = Vec::with_capacity(command.paths.len());
        for source in &command.paths {
            created.push(self.vault_item_repository.import_item(
                &command.vault_path,
                source,
                &command.dest_dir,
            )?);
        }
        Ok(created)
    }
}
