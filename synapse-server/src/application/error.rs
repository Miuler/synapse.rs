//! Errores de la capa de aplicación, independientes del transporte (gRPC/HTTP).

use thiserror::Error;

/// Error de aplicación que los entrypoints traducen al código de transporte correspondiente.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApplicationError {
    /// Sin token, token inválido o caducado.
    #[error("no autenticado: {0}")]
    Unauthenticated(String),

    /// Cuenta válida pero fuera de la lista de cuentas permitidas.
    #[error("permiso denegado: {0}")]
    PermissionDenied(String),

    /// Servicio externo temporalmente inaccesible (p. ej. JWKS caducado sin conexión a Google).
    #[error("servicio no disponible: {0}")]
    Unavailable(String),

    /// Recurso inexistente o perteneciente a otro tenant (respuesta idéntica).
    #[error("no encontrado: {0}")]
    NotFound(String),

    /// Argumento, ruta, hash o delta mal formado.
    #[error("argumento inválido en '{field}': {reason}")]
    InvalidArgument { field: String, reason: String },

    /// Faltan contenidos referenciados por un delta o manifiesto.
    #[error("faltan contenidos en el almacén")]
    MissingContent { missing_hashes: Vec<String> },

    /// La raíz Merkle declarada no coincide con la recalculada por el servidor.
    #[error("la raíz declarada no coincide con la calculada")]
    RootMismatch,

    /// El `base_root_hash` enviado ya no es el vigente.
    #[error("la instantánea base está desactualizada")]
    StaleBase { current_root_hash: String },

    /// Límite de tamaño o de nodos superado.
    #[error("límite superado: {0}")]
    ResourceExhausted(String),

    /// Error interno inesperado.
    #[error("error interno: {0}")]
    Internal(String),
}
