use crate::application::dto::version::push_versions_command::PushVersionsCommand;
use crate::domain::repository::version_control::VersionControl;

/// Publica las versiones locales en el remoto.
#[derive(Clone)]
pub struct PushVersionsUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> PushVersionsUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, command: PushVersionsCommand) -> Result<String, String> {
        self.versions.push(&command.vault_path)
    }
}
