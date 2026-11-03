use crate::application::dto::version::get_vault_version_state_query::GetVaultVersionStateQuery;
use crate::domain::version::version::VaultVersionState;
use crate::domain::version::version_control::VersionControl;

/// Estado de versionado de la bóveda.
#[derive(Clone)]
pub struct GetVaultVersionStateUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> GetVaultVersionStateUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: GetVaultVersionStateQuery) -> Result<VaultVersionState, String> {
        self.version_control.get_vault_status(&query.vault_path)
    }
}
