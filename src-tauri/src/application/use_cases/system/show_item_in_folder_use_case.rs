use crate::application::dto::system::show_item_in_folder_command::ShowItemInFolderCommand;
use crate::application::service::system_shell::SystemShell;

/// Muestra y resalta un archivo o carpeta de la bóveda en el explorador nativo del sistema.
#[derive(Clone)]
pub struct ShowItemInFolderUseCase<A: SystemShell> {
    shell: A,
}

impl<A: SystemShell> ShowItemInFolderUseCase<A> {
    pub fn new(shell: A) -> Self {
        Self { shell }
    }

    pub fn execute(&self, command: ShowItemInFolderCommand) -> Result<(), String> {
        self.shell.show_in_folder(&command.vault_path, &command.relative_path)
    }
}
