use std::path::PathBuf;

pub struct ImportExternalItemsCommand {
    /// Bóveda activa.
    pub vault_path: PathBuf,
    /// Rutas absolutas fuera de la bóveda (o de otra bóveda).
    pub paths: Vec<PathBuf>,
    pub dest_dir: String,
}
