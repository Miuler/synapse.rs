use crate::application::dto::vault_item::delete_vault_item_command::DeleteVaultItemCommand;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Elimina un archivo o carpeta de la bóveda.
#[derive(Clone)]
pub struct DeleteVaultItemUseCase<A: VaultItemRepository> {
    items: A,
}

impl<A: VaultItemRepository> DeleteVaultItemUseCase<A> {
    pub fn new(items: A) -> Self {
        Self { items }
    }

    pub fn execute(&self, command: DeleteVaultItemCommand) -> Result<(), String> {
        self.items.delete_item(&command.vault_path, &command.relative_path)
    }
}
