use crate::application::dto::vault_item::create_folder_command::CreateFolderCommand;
use crate::domain::vault::vault_item_repository::VaultItemRepository;

/// Crea una carpeta dentro de la bóveda.
#[derive(Clone)]
pub struct CreateFolderUseCase<A: VaultItemRepository> {
    vault_item_repository: A,
}

impl<A: VaultItemRepository> CreateFolderUseCase<A> {
    pub fn new(vault_item_repository: A) -> Self {
        Self {
            vault_item_repository,
        }
    }

    pub fn execute(&self, command: CreateFolderCommand) -> Result<(), String> {
        self.vault_item_repository
            .create_folder(&command.vault_path, &command.relative_path)
    }
}
