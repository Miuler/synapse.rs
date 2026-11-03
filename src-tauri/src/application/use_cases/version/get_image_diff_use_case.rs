use crate::application::dto::version::get_image_diff_query::GetImageDiffQuery;
use crate::domain::version::version::ImageDiff;
use crate::domain::version::version_control::VersionControl;

/// Diferencias de una imagen entre versiones.
#[derive(Clone)]
pub struct GetImageDiffUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> GetImageDiffUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: GetImageDiffQuery) -> Result<ImageDiff, String> {
        self.version_control.get_image_diff(
            &query.vault_path,
            &query.path,
            query.staged,
            query.revision.as_deref(),
        )
    }
}
