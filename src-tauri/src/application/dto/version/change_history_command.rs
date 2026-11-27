use crate::domain::version::history_change::HistoryAction;
use std::path::PathBuf;

pub struct ChangeHistoryCommand {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    pub action: HistoryAction,
}
