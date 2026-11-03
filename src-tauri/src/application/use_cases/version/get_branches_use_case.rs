use crate::application::dto::version::get_branches_query::GetBranchesQuery;
use crate::domain::version::version::BranchList;
use crate::domain::version::version_control::VersionControl;

/// Ramas de la bóveda.
#[derive(Clone)]
pub struct GetBranchesUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> GetBranchesUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: GetBranchesQuery) -> Result<BranchList, String> {
        self.version_control.get_branches(&query.vault_path)
    }
}
