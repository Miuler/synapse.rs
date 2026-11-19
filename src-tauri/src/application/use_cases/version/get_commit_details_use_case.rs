use crate::application::dto::version::get_commit_details_query::GetCommitDetailsQuery;
use crate::domain::version::version::CommitDetails;
use crate::domain::version::version_control::VersionControl;

/// Detalles y archivos cambiados de un commit.
#[derive(Clone)]
pub struct GetCommitDetailsUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> GetCommitDetailsUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: GetCommitDetailsQuery) -> Result<CommitDetails, String> {
        self.version_control.get_commit_details(&query.vault_path, &query.commit)
    }
}
