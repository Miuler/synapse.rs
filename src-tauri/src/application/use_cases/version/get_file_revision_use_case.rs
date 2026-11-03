use crate::application::dto::version::get_file_revision_query::GetFileRevisionQuery;
use crate::domain::version::version::FileRevision;
use crate::domain::version::version_control::VersionControl;

/// Contenido de un archivo en una versión. Sin versión explícita usa la preparada (staged) o la última (HEAD).
#[derive(Clone)]
pub struct GetFileRevisionUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> GetFileRevisionUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: GetFileRevisionQuery) -> Result<FileRevision, String> {
        let default_rev = if query.staged { "staged" } else { "HEAD" };
        let rev = query.revision.as_deref().unwrap_or(default_rev);
        self.version_control
            .get_file_revision(&query.vault_path, &query.path, rev)
    }
}
