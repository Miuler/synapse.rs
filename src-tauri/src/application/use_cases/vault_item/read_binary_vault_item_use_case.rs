use crate::application::dto::vault_item::read_binary_vault_item_query::ReadBinaryVaultItemQuery;
use crate::domain::repository::vault_item_repository::VaultItemRepository;

/// Lee el contenido binario de un archivo de la bóveda.
#[derive(Clone)]
pub struct ReadBinaryVaultItemUseCase<A: VaultItemRepository> {
    items: A,
}

impl<A: VaultItemRepository> ReadBinaryVaultItemUseCase<A> {
    pub fn new(items: A) -> Self {
        Self { items }
    }

    pub fn execute(&self, query: ReadBinaryVaultItemQuery) -> Result<Vec<u8>, String> {
        self.items.read_binary_item(&query.vault_path, &query.relative_path)
    }
}
