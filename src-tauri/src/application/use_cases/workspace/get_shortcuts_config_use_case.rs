use crate::application::dto::workspace::get_shortcuts_config_query::GetShortcutsConfigQuery;
use crate::domain::config::config::ShortcutsConfig;
use crate::domain::workspace::workspace_registry_repository::WorkspaceRegistryRepository;

/// Configuración de atajos de teclado.
#[derive(Clone)]
pub struct GetShortcutsConfigUseCase<A: WorkspaceRegistryRepository> {
    workspace_registry_repository: A,
}

impl<A: WorkspaceRegistryRepository> GetShortcutsConfigUseCase<A> {
    pub fn new(workspace_registry_repository: A) -> Self {
        Self {
            workspace_registry_repository,
        }
    }

    pub fn execute(&self, _query: GetShortcutsConfigQuery) -> Result<ShortcutsConfig, String> {
        self.workspace_registry_repository.load_shortcuts_config()
    }
}
