use crate::domain::issue::connection::Connection;
use serde::Serialize;

/// Lo que la interfaz puede saber de una conexión: nunca su token.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConnectionView {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub server: String,
    pub user: Option<String>,
    /// Referencia de 1Password configurada.
    pub reference: Option<String>,
    /// Hay un token guardado en el llavero.
    pub stored: bool,
}

impl ConnectionView {
    pub fn new(connection: Connection, stored: bool) -> Self {
        Self {
            // Algo escrito a mano que no sea una referencia (un token) no sale de aquí
            reference: connection.secret.filter(|s| s.starts_with("op://")),
            id: connection.id,
            provider: connection.provider,
            name: connection.name,
            server: connection.server,
            user: connection.user,
            stored,
        }
    }
}
