use crate::application::dto::version::get_vault_version_state_query::GetVaultVersionStateQuery;
use crate::domain::version::version_control::VersionControl;
use crate::domain::version::version::VaultVersionState;

/// Estado de versionado de la bóveda.
#[derive(Clone)]
pub struct GetVaultVersionStateUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> GetVaultVersionStateUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, query: GetVaultVersionStateQuery) -> Result<VaultVersionState, String> {
        self.versions.get_vault_status(&query.vault_path)
    }
}
