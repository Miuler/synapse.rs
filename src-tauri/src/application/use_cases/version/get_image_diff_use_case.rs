use crate::application::dto::version::get_image_diff_query::GetImageDiffQuery;
use crate::domain::repository::version_control::VersionControl;
use crate::domain::model::version::ImageDiff;

/// Diferencias de una imagen entre versiones.
#[derive(Clone)]
pub struct GetImageDiffUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> GetImageDiffUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, query: GetImageDiffQuery) -> Result<ImageDiff, String> {
        self.versions.get_image_diff(&query.vault_path, &query.path, query.staged, query.revision.as_deref())
    }
}
