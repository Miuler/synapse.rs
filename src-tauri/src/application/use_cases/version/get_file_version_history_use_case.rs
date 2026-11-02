use crate::application::dto::version::get_file_version_history_query::GetFileVersionHistoryQuery;
use crate::domain::version::version_control::VersionControl;
use crate::domain::version::version::FileVersionHistory;

/// Historial de versiones de un archivo.
#[derive(Clone)]
pub struct GetFileVersionHistoryUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> GetFileVersionHistoryUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, query: GetFileVersionHistoryQuery) -> Result<FileVersionHistory, String> {
        self.versions.get_file_log(&query.vault_path, &query.path, query.max_count)
    }
}
