use crate::application::dto::vault_item::move_vault_items_command::MoveVaultItemsCommand;
use crate::domain::repository::vault_item_repository::VaultItemRepository;
use crate::domain::repository::version_control::VersionControl;
use crate::application::dto::vault_item::moved_vault_item::MovedVaultItem;

/// Mueve elementos a una carpeta. Si estaban versionados, deja el movimiento preparado en el versionado. Devuelve los pares origen/destino realmente movidos; el error solo se propaga cuando no se movió nada.
#[derive(Clone)]
pub struct MoveVaultItemsUseCase<A: VaultItemRepository, B: VersionControl> {
    items: A,
    versions: B,
}

impl<A: VaultItemRepository, B: VersionControl> MoveVaultItemsUseCase<A, B> {
    pub fn new(items: A, versions: B) -> Self {
        Self { items, versions }
    }

    pub fn execute(&self, command: MoveVaultItemsCommand) -> Result<Vec<MovedVaultItem>, String> {
        let mut moved = Vec::with_capacity(command.paths.len());
        let mut first_error: Option<String> = None;
        for source in &command.paths {
            let was_tracked = self.versions.is_tracked(&command.vault_path, source);
            match self.items.move_item(&command.vault_path, source, &command.dest_dir) {
                Ok(to) if &to != source => {
                    if was_tracked {
                        if let Err(err) = self.versions.track(&command.vault_path, &[source.clone(), to.clone()]) {
                            log::warn!("[move_vault_items] No se pudo preparar el movimiento en git: {}", err);
                        }
                    }
                    moved.push(MovedVaultItem { from: source.clone(), to });
                }
                Ok(_) => {}
                Err(err) => {
                    first_error.get_or_insert(err);
                }
            }
        }
        // Si algo se movió se devuelve lo hecho; el error solo se propaga cuando no se movió nada
        match (moved.is_empty(), first_error) {
            (true, Some(err)) => Err(err),
            _ => Ok(moved),
        }
    }
}
