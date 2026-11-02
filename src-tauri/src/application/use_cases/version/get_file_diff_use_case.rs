use crate::application::dto::version::get_file_diff_query::GetFileDiffQuery;
use crate::domain::version::version_control::VersionControl;
use crate::domain::version::version::FileDiff;

/// Diferencias de texto de un archivo.
#[derive(Clone)]
pub struct GetFileDiffUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> GetFileDiffUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, query: GetFileDiffQuery) -> Result<FileDiff, String> {
        self.versions.diff(&query.vault_path, &query.path, query.staged, query.revision.as_deref())
    }
}
