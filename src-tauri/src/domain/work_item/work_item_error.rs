use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemErrorKind {
    MissingToken,
    /// El llavero del sistema no respondió.
    Keyring,
    /// La referencia configurada para el token no es de la forma `op://…`.
    TokenReference,
    /// 1Password no entregó el token.
    OnePassword,
    /// La URL del proyecto apunta a un servidor al que no se envían credenciales.
    HostNotAllowed,
    InvalidUrl,
    Auth,
    NotFound,
    /// El work item cambió desde que se leyó.
    Conflict,
    /// El gestor rechaza el valor (por ejemplo, un estado que no existe).
    Rule,
    Request,
}

/// Se entrega a la interfaz como clase de error y detalle: el texto visible lo pone ella.
#[derive(Debug, Clone, Serialize)]
pub struct WorkItemError {
    pub kind: WorkItemErrorKind,
    pub detail: String,
}

impl WorkItemError {
    pub fn new(kind: WorkItemErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}
