use crate::application::dto::vault_item::create_folder_command::CreateFolderCommand;
use crate::domain::repository::vault_item_repository::VaultItemRepository;

/// Crea una carpeta dentro de la bóveda.
#[derive(Clone)]
pub struct CreateFolderUseCase<A: VaultItemRepository> {
    items: A,
}

impl<A: VaultItemRepository> CreateFolderUseCase<A> {
    pub fn new(items: A) -> Self {
        Self { items }
    }

    pub fn execute(&self, command: CreateFolderCommand) -> Result<(), String> {
        self.items.create_folder(&command.vault_path, &command.relative_path)
    }
}
