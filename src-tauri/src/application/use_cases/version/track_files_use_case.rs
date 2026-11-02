use crate::application::dto::version::track_files_command::TrackFilesCommand;
use crate::domain::version::version_control::VersionControl;

/// Prepara archivos para la próxima versión.
#[derive(Clone)]
pub struct TrackFilesUseCase<A: VersionControl> {
    versions: A,
}

impl<A: VersionControl> TrackFilesUseCase<A> {
    pub fn new(versions: A) -> Self {
        Self { versions }
    }

    pub fn execute(&self, command: TrackFilesCommand) -> Result<(), String> {
        self.versions.track(&command.vault_path, &command.paths)
    }
}
