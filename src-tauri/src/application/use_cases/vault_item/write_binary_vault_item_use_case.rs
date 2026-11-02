use crate::application::dto::vault_item::write_binary_vault_item_command::WriteBinaryVaultItemCommand;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Escribe contenido binario en un archivo de la bóveda.
#[derive(Clone)]
pub struct WriteBinaryVaultItemUseCase<A: VaultItemRepository> {
    items: A,
}

impl<A: VaultItemRepository> WriteBinaryVaultItemUseCase<A> {
    pub fn new(items: A) -> Self {
        Self { items }
    }

    pub fn execute(&self, command: WriteBinaryVaultItemCommand) -> Result<(), String> {
        self.items.write_binary_item(&command.vault_path, &command.relative_path, &command.bytes)
    }
}
