use crate::domain::model::workspace_session::OpenTabDto;

pub struct SaveOpenTabsCommand {
    pub tabs: Vec<OpenTabDto>,
    pub active_tab: Option<String>,
}
