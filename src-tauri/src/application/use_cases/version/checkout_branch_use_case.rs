use crate::application::dto::version::checkout_branch_command::CheckoutBranchCommand;
use crate::domain::version::version_control::VersionControl;

/// Cambia a otra rama.
#[derive(Clone)]
pub struct CheckoutBranchUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> CheckoutBranchUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, command: CheckoutBranchCommand) -> Result<String, String> {
        self.version_control
            .checkout_branch(&command.vault_path, &command.branch_name)
    }
}
