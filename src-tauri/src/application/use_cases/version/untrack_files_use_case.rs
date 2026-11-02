use crate::application::dto::version::untrack_files_command::UntrackFilesCommand;
use crate::domain::repository::version_control::VersionControl;

/// Quita archivos de la preparación de la próxima versión.
#[derive(Clone)]
pub struct UntrackFilesUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> UntrackFilesUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, command: UntrackFilesCommand) -> Result<(), String> {
        self.versions.untrack(&command.vault_path, &command.paths)
    }
}
