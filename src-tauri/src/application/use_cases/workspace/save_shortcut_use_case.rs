use crate::application::dto::workspace::save_shortcut_command::SaveShortcutCommand;
use crate::domain::config::config::ShortcutsConfig;
use crate::domain::workspace::workspace_registry_repository::WorkspaceRegistryRepository;

/// Guarda un atajo de teclado y devuelve la configuración resultante.
#[derive(Clone)]
pub struct SaveShortcutUseCase<A: WorkspaceRegistryRepository> {
    workspace_registry_repository: A,
}

impl<A: WorkspaceRegistryRepository> SaveShortcutUseCase<A> {
    pub fn new(workspace_registry_repository: A) -> Self {
        Self {
            workspace_registry_repository,
        }
    }

    pub fn execute(&self, command: SaveShortcutCommand) -> Result<ShortcutsConfig, String> {
        if command.section != "welcome" && command.section != "workspace" {
            return Err(format!("Sección de atajos desconocida: {}", command.section));
        }
        let value = command.value.trim();
        if value.is_empty() {
            return Err("El atajo no puede estar vacío".to_string());
        }
        self.workspace_registry_repository
            .save_shortcut(&command.section, &command.action, value)
    }
}
