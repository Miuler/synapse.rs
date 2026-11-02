/// Cambios detectados en los archivos de la bóveda.
pub struct IndexVaultChangeCommand {
    /// Rutas relativas de archivos añadidos o modificados.
    pub upserted: Vec<String>,
    /// Rutas relativas (archivos o carpetas) eliminadas.
    pub removed: Vec<String>,
}
