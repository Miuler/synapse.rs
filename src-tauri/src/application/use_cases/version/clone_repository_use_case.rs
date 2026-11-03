use crate::application::dto::version::clone_repository_command::CloneRepositoryCommand;
use crate::domain::version::version_control::VersionControl;

/// Clona un repositorio remoto en una carpeta local.
#[derive(Clone)]
pub struct CloneRepositoryUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> CloneRepositoryUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, command: CloneRepositoryCommand) -> Result<(), String> {
        self.version_control
            .clone_repository(&command.repo_url, &command.target_path)
    }
}
