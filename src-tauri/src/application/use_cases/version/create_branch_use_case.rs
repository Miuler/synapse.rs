use crate::application::dto::version::create_branch_command::CreateBranchCommand;
use crate::domain::version::version_control::VersionControl;

/// Crea una rama a partir de otra y opcionalmente cambia a ella.
#[derive(Clone)]
pub struct CreateBranchUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> CreateBranchUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, command: CreateBranchCommand) -> Result<String, String> {
        self.version_control.create_branch(
            &command.vault_path,
            &command.new_branch,
            &command.base_branch,
            command.checkout,
        )
    }
}
