use crate::application::dto::version::untrack_files_command::UntrackFilesCommand;
use crate::domain::version::version_control::VersionControl;

/// Quita archivos de la preparación de la próxima versión.
#[derive(Clone)]
pub struct UntrackFilesUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> UntrackFilesUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, command: UntrackFilesCommand) -> Result<(), String> {
        self.version_control
            .untrack(&command.vault_path, &command.paths)
    }
}
