use crate::application::dto::open_file_with_default_app_command::OpenFileWithDefaultAppCommand;
use crate::application::service::system_shell::SystemShell;

/// Abre un archivo de la bóveda con la aplicación por defecto del sistema operativo.
pub struct OpenFileWithDefaultAppUseCase<S: SystemShell> {
    shell: S,
}

impl<S: SystemShell> OpenFileWithDefaultAppUseCase<S> {
    pub fn new(shell: S) -> Self {
        Self { shell }
    }

    pub fn execute(&self, command: OpenFileWithDefaultAppCommand) -> Result<(), String> {
        self.shell.open_with_default_app(&command.vault_path, &command.relative_path)
    }
}
