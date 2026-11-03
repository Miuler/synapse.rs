use crate::application::dto::version::push_versions_command::PushVersionsCommand;
use crate::domain::version::version_control::VersionControl;

/// Publica las versiones locales en el remoto.
#[derive(Clone)]
pub struct PushVersionsUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> PushVersionsUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, command: PushVersionsCommand) -> Result<String, String> {
        self.version_control.push(&command.vault_path)
    }
}
