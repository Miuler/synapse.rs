use crate::application::dto::version::save_version_command::SaveVersionCommand;
use crate::domain::version::version_control::VersionControl;

/// Guarda una versión con los archivos indicados.
#[derive(Clone)]
pub struct SaveVersionUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> SaveVersionUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, command: SaveVersionCommand) -> Result<String, String> {
        self.versions.save_version(&command.vault_path, &command.paths, &command.message)
    }
}
