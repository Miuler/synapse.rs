use crate::application::dto::version::get_file_version_history_query::GetFileVersionHistoryQuery;
use crate::domain::version::version::FileVersionHistory;
use crate::domain::version::version_control::VersionControl;

/// Historial de versiones de un archivo.
#[derive(Clone)]
pub struct GetFileVersionHistoryUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> GetFileVersionHistoryUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: GetFileVersionHistoryQuery) -> Result<FileVersionHistory, String> {
        self.version_control
            .get_file_log(&query.vault_path, &query.path, query.max_count)
    }
}
