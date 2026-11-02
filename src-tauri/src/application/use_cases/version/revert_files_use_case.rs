use crate::application::dto::version::revert_files_command::RevertFilesCommand;
use crate::domain::repository::version_control::VersionControl;

/// Descarta los cambios de archivos y vuelve a su última versión.
#[derive(Clone)]
pub struct RevertFilesUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> RevertFilesUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, command: RevertFilesCommand) -> Result<(), String> {
        self.versions.revert(&command.vault_path, &command.paths)
    }
}
