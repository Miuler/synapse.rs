use crate::application::dto::version::create_branch_command::CreateBranchCommand;
use crate::domain::repository::version_control::VersionControl;

/// Crea una rama a partir de otra y opcionalmente cambia a ella.
#[derive(Clone)]
pub struct CreateBranchUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> CreateBranchUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, command: CreateBranchCommand) -> Result<String, String> {
        self.versions.create_branch(&command.vault_path, &command.new_branch, &command.base_branch, command.checkout)
    }
}
