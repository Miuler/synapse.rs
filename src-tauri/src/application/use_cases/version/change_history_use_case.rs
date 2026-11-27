use crate::application::dto::version::change_history_command::ChangeHistoryCommand;
use crate::domain::version::history_change::UndoPoint;
use crate::domain::version::version_control::VersionControl;

/// Squash, revert o reset de la rama actual, y su deshacer.
#[derive(Clone)]
pub struct ChangeHistoryUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> ChangeHistoryUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, cmd: ChangeHistoryCommand) -> Result<Option<UndoPoint>, String> {
        self.version_control
            .change_history(&cmd.vault_path, &cmd.action)
    }
}
