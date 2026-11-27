use crate::application::dto::version::merge_command::MergeCommand;
use crate::application::dto::version::merge_query::{ConflictVersionsQuery, MergeStateQuery};
use crate::domain::version::merge::{ConflictVersions, MergeOutcome, MergeState};
use crate::domain::version::version_control::VersionControl;

/// La fusión de otra rama en la actual: consultarla y avanzarla.
#[derive(Clone)]
pub struct MergeUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> MergeUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn state(&self, query: MergeStateQuery) -> Result<Option<MergeState>, String> {
        self.version_control.merge_state(&query.vault_path)
    }

    pub fn versions(&self, query: ConflictVersionsQuery) -> Result<ConflictVersions, String> {
        self.version_control.conflict_versions(&query.vault_path, &query.path)
    }

    pub fn execute(&self, cmd: MergeCommand) -> Result<MergeOutcome, String> {
        self.version_control.merge(&cmd.vault_path, &cmd.action)
    }
}
