use crate::application::dto::version::get_file_revision_query::GetFileRevisionQuery;
use crate::domain::repository::version_control::VersionControl;
use crate::domain::model::version::FileRevision;

/// Contenido de un archivo en una versión. Sin versión explícita usa la preparada (staged) o la última (HEAD).
#[derive(Clone)]
pub struct GetFileRevisionUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> GetFileRevisionUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, query: GetFileRevisionQuery) -> Result<FileRevision, String> {
        let default_rev = if query.staged { "staged" } else { "HEAD" };
        let rev = query.revision.as_deref().unwrap_or(default_rev);
        self.versions.get_file_revision(&query.vault_path, &query.path, rev)
    }
}
