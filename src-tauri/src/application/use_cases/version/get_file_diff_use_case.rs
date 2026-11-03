use crate::application::dto::version::get_file_diff_query::GetFileDiffQuery;
use crate::domain::version::version::FileDiff;
use crate::domain::version::version_control::VersionControl;

/// Diferencias de texto de un archivo.
#[derive(Clone)]
pub struct GetFileDiffUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> GetFileDiffUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: GetFileDiffQuery) -> Result<FileDiff, String> {
        self.version_control.diff(
            &query.vault_path,
            &query.path,
            query.staged,
            query.revision.as_deref(),
        )
    }
}
