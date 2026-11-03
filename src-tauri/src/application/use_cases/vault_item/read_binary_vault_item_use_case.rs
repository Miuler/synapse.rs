use crate::application::dto::vault_item::read_binary_vault_item_query::ReadBinaryVaultItemQuery;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Lee el contenido binario de un archivo de la bóveda.
#[derive(Clone)]
pub struct ReadBinaryVaultItemUseCase<A: VaultItemRepository> {
    vault_item_repository: A,
}

impl<A: VaultItemRepository> ReadBinaryVaultItemUseCase<A> {
    pub fn new(vault_item_repository: A) -> Self {
        Self {
            vault_item_repository,
        }
    }

    pub fn execute(&self, query: ReadBinaryVaultItemQuery) -> Result<Vec<u8>, String> {
        self.vault_item_repository
            .read_binary_item(&query.vault_path, &query.relative_path)
    }
}
