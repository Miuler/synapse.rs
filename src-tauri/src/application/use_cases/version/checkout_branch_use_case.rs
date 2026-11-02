use crate::application::dto::version::checkout_branch_command::CheckoutBranchCommand;
use crate::domain::version::version_control::VersionControl;

/// Cambia a otra rama.
#[derive(Clone)]
pub struct CheckoutBranchUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> CheckoutBranchUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, command: CheckoutBranchCommand) -> Result<String, String> {
        self.versions.checkout_branch(&command.vault_path, &command.branch_name)
    }
}
