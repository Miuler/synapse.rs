use crate::application::dto::version::get_repo_log_query::GetRepoLogQuery;
use crate::domain::version::version::RepoLog;
use crate::domain::version::version_control::VersionControl;

/// Log de todas las ramas del repositorio.
#[derive(Clone)]
pub struct GetRepoLogUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> GetRepoLogUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: GetRepoLogQuery) -> Result<RepoLog, String> {
        self.version_control.get_repo_log(&query.vault_path, query.max_count)
    }
}
