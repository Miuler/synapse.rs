use crate::domain::version::merge::MergeAction;
use std::path::PathBuf;

pub struct MergeCommand {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    pub action: MergeAction,
}
