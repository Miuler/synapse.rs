use std::path::PathBuf;

/// Una bóveda que el usuario tiene (abierta en una ventana o entre sus recientes).
#[derive(Debug, Clone, PartialEq)]
pub struct VaultRef {
    pub name: String,
    pub path: PathBuf,
    /// Abierta ahora en alguna ventana de Synapse.
    pub open: bool,
}

/// Bóvedas del usuario que un agente puede consultar desde otra bóveda.
pub trait VaultCatalog: Send + Sync {
    fn list(&self) -> Vec<VaultRef>;
}

/// Pregunta al usuario y devuelve su respuesta (por ejemplo, si una bóveda puede consultar otra).
pub trait UserPrompter: Send + Sync {
    fn confirm(&self, title: &str, message: &str) -> Result<bool, String>;
}
