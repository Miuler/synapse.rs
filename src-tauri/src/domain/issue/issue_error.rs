use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueErrorKind {
    MissingToken,
    /// El llavero del sistema no respondió.
    Keyring,
    /// La referencia configurada para el token no es de la forma `op://…`.
    TokenReference,
    /// 1Password no entregó el token.
    OnePassword,
    /// El ámbito apunta a un servidor al que no se envían credenciales.
    HostNotAllowed,
    InvalidUrl,
    Auth,
    NotFound,
    /// La tarea cambió desde que se leyó.
    Conflict,
    /// El gestor rechaza el valor (por ejemplo, un estado que no existe).
    Rule,
    Request,
    /// La conexión pide un proveedor que esta versión no tiene.
    UnknownProvider,
}

/// Se entrega a la interfaz como clase de error y detalle: el texto visible lo pone ella.
#[derive(Debug, Clone, Serialize)]
pub struct IssueError {
    pub kind: IssueErrorKind,
    pub detail: String,
}

impl IssueError {
    pub fn new(kind: IssueErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}
