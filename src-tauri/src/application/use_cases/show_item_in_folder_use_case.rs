use crate::application::dto::show_item_in_folder_command::ShowItemInFolderCommand;
use crate::application::service::system_shell::SystemShell;

/// Muestra y resalta un archivo o carpeta de la bóveda en el explorador nativo del sistema.
pub struct ShowItemInFolderUseCase<S: SystemShell> {
    shell: S,
}

impl<S: SystemShell> ShowItemInFolderUseCase<S> {
    pub fn new(shell: S) -> Self {
        Self { shell }
    }

    pub fn execute(&self, command: ShowItemInFolderCommand) -> Result<(), String> {
        self.shell.show_in_folder(&command.vault_path, &command.relative_path)
    }
}
