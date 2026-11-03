use crate::application::dto::system::show_item_in_folder_command::ShowItemInFolderCommand;
use crate::application::service::system_shell::SystemShell;

/// Muestra y resalta un archivo o carpeta de la bóveda en el explorador nativo del sistema.
#[derive(Clone)]
pub struct ShowItemInFolderUseCase<A: SystemShell> {
    system_shell: A,
}

impl<A: SystemShell> ShowItemInFolderUseCase<A> {
    pub fn new(system_shell: A) -> Self {
        Self { system_shell }
    }

    pub fn execute(&self, command: ShowItemInFolderCommand) -> Result<(), String> {
        self.system_shell
            .show_in_folder(&command.vault_path, &command.relative_path)
    }
}
