use serde::{Deserialize, Serialize};

/// Opción que el agente ofrece al pedir permiso (permitir una vez, siempre, rechazar…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOption {
    pub id: String,
    pub name: String,
    pub kind: String,
}

/// El agente pide permiso para usar una herramienta o modificar un archivo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub session_id: String,
    pub request_id: String,
    pub title: String,
    pub options: Vec<PermissionOption>,
}

/// Respuesta del usuario a una petición de permiso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum PermissionDecision {
    Selected { option_id: String },
    Cancelled,
}
