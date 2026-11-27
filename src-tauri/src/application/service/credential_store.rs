use crate::domain::issue::issue_error::IssueError;

/// Dónde se guardan los tokens (el llavero del sistema). Las llamadas pueden bloquear.
pub trait CredentialStore: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<String>, String>;
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    /// Borrar lo que no está no es un error.
    fn delete(&self, name: &str) -> Result<(), String>;
}

/// La credencial única anterior a las conexiones.
pub trait GlobalCredential: Send + Sync {
    /// El token o la referencia `op://` que valía para `server`. Solo se entrega para los servidores
    /// a los que ya se enviaba: así nadie puede copiarla a una conexión que apunte a otro sitio.
    fn for_server(&self, provider: &str, server: &str) -> Result<Option<String>, IssueError>;
}
