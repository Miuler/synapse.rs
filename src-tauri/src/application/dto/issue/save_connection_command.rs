use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SaveConnectionCommand {
    /// Sin id, la conexión es nueva.
    pub id: Option<String>,
    pub provider: String,
    pub name: String,
    pub server: String,
    #[serde(default)]
    pub user: Option<String>,
    /// `None` la deja como está; vacía, la quita; una referencia `op://` o un token.
    #[serde(default)]
    pub credential: Option<String>,
    /// Copiar la credencial única anterior, si vale para este servidor.
    #[serde(default)]
    pub use_global: bool,
}
