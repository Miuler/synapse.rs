use serde_json::Value;

/// Otras instancias de Synapse abiertas (otras bóvedas), alcanzables por su canal `/peer`.
pub trait PeerInstances: Send + Sync {
    /// Pregunta lo mismo a todas: `[{vault, path, result | error}]`.
    fn broadcast(&self, request: Value) -> Vec<Value>;
    /// Envía un pedido a la instancia de la bóveda `vault` (nombre o ruta).
    fn send(&self, vault: &str, request: Value) -> Result<Value, String>;
}
