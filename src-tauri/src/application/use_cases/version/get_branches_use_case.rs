use crate::application::dto::version::get_branches_query::GetBranchesQuery;
use crate::domain::repository::version_control::VersionControl;
use crate::domain::model::version::BranchList;

/// Ramas de la bóveda.
#[derive(Clone)]
pub struct GetBranchesUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> GetBranchesUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, query: GetBranchesQuery) -> Result<BranchList, String> {
        self.versions.get_branches(&query.vault_path)
    }
}
