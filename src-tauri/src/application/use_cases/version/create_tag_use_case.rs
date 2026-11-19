use crate::application::dto::version::create_tag_command::CreateTagCommand;
use crate::domain::version::version_control::VersionControl;

/// Crea un tag en un commit.
#[derive(Clone)]
pub struct CreateTagUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> CreateTagUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, cmd: CreateTagCommand) -> Result<String, String> {
        let message = cmd.message.as_deref().map(str::trim).filter(|m| !m.is_empty());
        self.version_control.create_tag(&cmd.vault_path, cmd.name.trim(), &cmd.commit, message)
    }
}
