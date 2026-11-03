use crate::application::dto::version::revert_files_command::RevertFilesCommand;
use crate::domain::version::version_control::VersionControl;

/// Descarta los cambios de archivos y vuelve a su última versión.
#[derive(Clone)]
pub struct RevertFilesUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> RevertFilesUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, command: RevertFilesCommand) -> Result<(), String> {
        self.version_control
            .revert(&command.vault_path, &command.paths)
    }
}
