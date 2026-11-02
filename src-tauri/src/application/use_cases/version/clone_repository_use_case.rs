use crate::application::dto::version::clone_repository_command::CloneRepositoryCommand;
use crate::domain::version::version_control::VersionControl;

/// Clona un repositorio remoto en una carpeta local.
#[derive(Clone)]
pub struct CloneRepositoryUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> CloneRepositoryUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, command: CloneRepositoryCommand) -> Result<(), String> {
        self.versions.clone_repository(&command.repo_url, &command.target_path)
    }
}
