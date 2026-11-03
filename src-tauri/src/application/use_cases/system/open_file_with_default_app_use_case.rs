use crate::application::dto::system::open_file_with_default_app_command::OpenFileWithDefaultAppCommand;
use crate::application::service::system_shell::SystemShell;

/// Abre un archivo de la bóveda con la aplicación por defecto del sistema operativo.
#[derive(Clone)]
pub struct OpenFileWithDefaultAppUseCase<A: SystemShell> {
    system_shell: A,
}

impl<A: SystemShell> OpenFileWithDefaultAppUseCase<A> {
    pub fn new(system_shell: A) -> Self {
        Self { system_shell }
    }

    pub fn execute(&self, command: OpenFileWithDefaultAppCommand) -> Result<(), String> {
        self.system_shell
            .open_with_default_app(&command.vault_path, &command.relative_path)
    }
}
