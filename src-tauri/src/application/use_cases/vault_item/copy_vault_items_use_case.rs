use crate::application::dto::vault_item::copy_vault_items_command::CopyVaultItemsCommand;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Pega (copia) elementos dentro de una carpeta y devuelve las rutas creadas.
#[derive(Clone)]
pub struct CopyVaultItemsUseCase<A: VaultItemRepository> {
    items: A,
}

impl<A: VaultItemRepository> CopyVaultItemsUseCase<A> {
    pub fn new(items: A) -> Self {
        Self { items }
    }

    pub fn execute(&self, command: CopyVaultItemsCommand) -> Result<Vec<String>, String> {
        let mut created = Vec::with_capacity(command.paths.len());
        for source in &command.paths {
            created.push(self.items.copy_item(&command.vault_path, source, &command.dest_dir)?);
        }
        Ok(created)
    }
}
