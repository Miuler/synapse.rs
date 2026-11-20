use std::path::PathBuf;

pub struct FastForwardCommand {
    /// Carpeta del repositorio.
    pub vault_path: PathBuf,
    /// Rama local que avanza
    pub branch: String,
    /// Commit o rama hasta donde avanza (p. ej. `origin/main`)
    pub target: String,
    /// Solo comprobar si se puede
    pub dry_run: bool,
}
