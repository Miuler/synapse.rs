use crate::application::dto::workspace::get_shortcuts_config_query::GetShortcutsConfigQuery;
use crate::domain::repository::workspace_registry_repository::WorkspaceRegistryRepository;
use crate::domain::model::config::ShortcutsConfig;

/// Configuración de atajos de teclado.
#[derive(Clone)]
pub struct GetShortcutsConfigUseCase<A: WorkspaceRegistryRepository> {
    workspaces: A,
}

impl<A: WorkspaceRegistryRepository> GetShortcutsConfigUseCase<A> {
    pub fn new(workspaces: A) -> Self {
        Self { workspaces }
    }

    pub fn execute(&self, _query: GetShortcutsConfigQuery) -> Result<ShortcutsConfig, String> {
        self.workspaces.load_shortcuts_config()
    }
}
