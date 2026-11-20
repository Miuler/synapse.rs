use crate::application::dto::version::fast_forward_command::FastForwardCommand;
use crate::domain::version::version_control::VersionControl;

/// Avanza una rama local hasta un commit o rama posterior (solo fast-forward).
#[derive(Clone)]
pub struct FastForwardUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> FastForwardUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, cmd: FastForwardCommand) -> Result<String, String> {
        self.version_control
            .fast_forward_branch(&cmd.vault_path, cmd.branch.trim(), cmd.target.trim(), cmd.dry_run)
    }
}
