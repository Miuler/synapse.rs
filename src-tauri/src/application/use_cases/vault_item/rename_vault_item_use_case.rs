use crate::application::dto::vault_item::rename_vault_item_command::RenameVaultItemCommand;
use crate::domain::vault::vault_item_repository::VaultItemRepository;
use crate::domain::version::version_control::VersionControl;

/// Renombra un archivo o carpeta. Si estaba versionado, deja el renombrado preparado en el versionado.
#[derive(Clone)]
pub struct RenameVaultItemUseCase<A: VaultItemRepository, B: VersionControl> {
    vault_item_repository: A,
    version_control: B,
}

impl<A: VaultItemRepository, B: VersionControl> RenameVaultItemUseCase<A, B> {
    pub fn new(vault_item_repository: A, version_control: B) -> Self {
        Self {
            vault_item_repository,
            version_control,
        }
    }

    pub fn execute(&self, command: RenameVaultItemCommand) -> Result<String, String> {
        // Si estaba versionado, el renombrado se deja preparado en el índice (git detecta el rename)
        let was_tracked = self
            .version_control
            .is_tracked(&command.vault_path, &command.relative_path);
        let renamed_path = self.vault_item_repository.rename_item(
            &command.vault_path,
            &command.relative_path,
            &command.new_name,
        )?;
        if was_tracked && renamed_path != command.relative_path {
            if let Err(err) = self.version_control.track(
                &command.vault_path,
                &[command.relative_path.clone(), renamed_path.clone()],
            ) {
                log::warn!(
                    "[rename_vault_item] No se pudo preparar el renombrado en git: {}",
                    err
                );
            }
        }
        Ok(renamed_path)
    }
}
