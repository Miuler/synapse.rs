use crate::domain::config::config::ShortcutsConfig;
use crate::domain::workspace::workspace_registry::{RecentWorkspaceEntry, WorkspaceRegistryConfig};

pub trait WorkspaceRegistryRepository: Send + Sync {
    fn load_registry(&self) -> Result<WorkspaceRegistryConfig, String>;
    fn save_registry(&self, config: &WorkspaceRegistryConfig) -> Result<(), String>;
    fn record_workspace_opened(
        &self,
        path: &str,
        notes_count: Option<usize>,
        git_branch: Option<String>,
        is_git_repo: Option<bool>,
    ) -> Result<RecentWorkspaceEntry, String>;
    fn toggle_pin(&self, path: &str) -> Result<Vec<RecentWorkspaceEntry>, String>;
    fn remove_workspace(&self, path: &str) -> Result<Vec<RecentWorkspaceEntry>, String>;
    fn list_workspaces(&self) -> Result<Vec<RecentWorkspaceEntry>, String>;
    fn load_shortcuts_config(&self) -> Result<ShortcutsConfig, String>;
    /// Cambia un atajo en `shortcuts.jsonc` conservando los comentarios del archivo.
    fn save_shortcut(
        &self,
        section: &str,
        action: &str,
        value: &str,
    ) -> Result<ShortcutsConfig, String>;
}
