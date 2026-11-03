use crate::application::dto::version::track_files_command::TrackFilesCommand;
use crate::domain::version::version_control::VersionControl;

/// Prepara archivos para la próxima versión.
#[derive(Clone)]
pub struct TrackFilesUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> TrackFilesUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, command: TrackFilesCommand) -> Result<(), String> {
        self.version_control
            .track(&command.vault_path, &command.paths)
    }
}
