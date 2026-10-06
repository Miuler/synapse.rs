/// Cambio detectado en los archivos del vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultChange {
    /// Rutas relativas de archivos añadidos o modificados.
    Upserted(Vec<String>),
    /// Rutas relativas (archivos o carpetas) eliminadas.
    Removed(Vec<String>),
}

/// Observador de cambios en el vault.
///
/// Permite que los componentes que detectan cambios (motor de navegación, watcher)
/// los publiquen sin conocer a sus consumidores (p. ej. el índice full-text).
/// Las implementaciones no deben bloquear: se invocan desde hilos de escaneo.
pub trait VaultChangeObserver: Send + Sync {
    fn on_vault_change(&self, change: VaultChange);
}
