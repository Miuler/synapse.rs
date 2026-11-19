use crate::application::dto::version::delete_ref_command::{DeleteRefCommand, RefToDelete};
use crate::domain::version::version_control::VersionControl;

/// Borra un tag o una rama local.
#[derive(Clone)]
pub struct DeleteRefUseCase<A: VersionControl> {
    version_control: A,
}

impl<A: VersionControl> DeleteRefUseCase<A> {
    pub fn new(version_control: A) -> Self {
        Self { version_control }
    }

    pub fn execute(&self, cmd: DeleteRefCommand) -> Result<String, String> {
        match cmd.target {
            RefToDelete::Tag(name) => self.version_control.delete_tag(&cmd.vault_path, &name),
            RefToDelete::Branch { name, force } => {
                self.version_control.delete_branch(&cmd.vault_path, &name, force)
            }
        }
    }
}
