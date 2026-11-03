use crate::application::dto::vault_item::delete_vault_item_command::DeleteVaultItemCommand;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Elimina un archivo o carpeta de la bóveda.
#[derive(Clone)]
pub struct DeleteVaultItemUseCase<A: VaultItemRepository> {
    vault_item_repository: A,
}

impl<A: VaultItemRepository> DeleteVaultItemUseCase<A> {
    pub fn new(vault_item_repository: A) -> Self {
        Self {
            vault_item_repository,
        }
    }

    pub fn execute(&self, command: DeleteVaultItemCommand) -> Result<(), String> {
        self.vault_item_repository
            .delete_item(&command.vault_path, &command.relative_path)
    }
}
