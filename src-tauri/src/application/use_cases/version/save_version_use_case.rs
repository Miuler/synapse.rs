use crate::application::dto::version::save_version_command::SaveVersionCommand;
use crate::domain::version::version_control::VersionControl;

/// Guarda una versión con los archivos indicados.
#[derive(Clone)]
pub struct SaveVersionUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> SaveVersionUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, command: SaveVersionCommand) -> Result<String, String> {
        self.version_control
            .save_version(&command.vault_path, &command.paths, &command.message)
    }
}
