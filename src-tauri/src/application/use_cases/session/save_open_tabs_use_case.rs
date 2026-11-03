use crate::application::dto::session::save_open_tabs_command::SaveOpenTabsCommand;
use crate::domain::workspace::workspace_session_repository::WorkspaceSessionRepository;

/// Guarda las pestañas abiertas y la activa.
#[derive(Clone)]
pub struct SaveOpenTabsUseCase<A: WorkspaceSessionRepository> {
    workspace_session_repository: A,
}

impl<A: WorkspaceSessionRepository> SaveOpenTabsUseCase<A> {
    pub fn new(workspace_session_repository: A) -> Self {
        Self {
            workspace_session_repository,
        }
    }

    pub fn execute(&self, command: SaveOpenTabsCommand) -> Result<(), String> {
        self.workspace_session_repository
            .save_open_tabs(&command.tabs, command.active_tab.as_deref())
    }
}
