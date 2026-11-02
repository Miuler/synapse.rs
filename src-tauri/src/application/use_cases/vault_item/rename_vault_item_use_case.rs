use crate::application::dto::vault_item::rename_vault_item_command::RenameVaultItemCommand;
use crate::domain::repository::vault_item_repository::VaultItemRepository;
use crate::domain::repository::version_control::VersionControl;

/// Renombra un archivo o carpeta. Si estaba versionado, deja el renombrado preparado en el versionado.
#[derive(Clone)]
pub struct RenameVaultItemUseCase<A: VaultItemRepository, B: VersionControl> {
    items: A,
    versions: B,
}

impl<A: VaultItemRepository, B: VersionControl> RenameVaultItemUseCase<A, B> {
    pub fn new(items: A, versions: B) -> Self {
        Self { items, versions }
    }

    pub fn execute(&self, command: RenameVaultItemCommand) -> Result<String, String> {
        // Si estaba versionado, el renombrado se deja preparado en el índice (git detecta el rename)
        let was_tracked = self.versions.is_tracked(&command.vault_path, &command.relative_path);
        let renamed_path = self.items.rename_item(&command.vault_path, &command.relative_path, &command.new_name)?;
        if was_tracked && renamed_path != command.relative_path {
            if let Err(err) = self
                .versions
                .track(&command.vault_path, &[command.relative_path.clone(), renamed_path.clone()])
            {
                log::warn!("[rename_vault_item] No se pudo preparar el renombrado en git: {}", err);
            }
        }
        Ok(renamed_path)
    }
}
