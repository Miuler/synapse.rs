use crate::application::dto::version::squash_preview_query::SquashPreviewQuery;
use crate::domain::version::history_change::SquashPreview;
use crate::domain::version::version_control::VersionControl;

/// Qué uniría un squash desde un commit.
#[derive(Clone)]
pub struct SquashPreviewUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> SquashPreviewUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, query: SquashPreviewQuery) -> Result<SquashPreview, String> {
        self.version_control
            .squash_preview(&query.vault_path, query.from.trim())
    }
}
