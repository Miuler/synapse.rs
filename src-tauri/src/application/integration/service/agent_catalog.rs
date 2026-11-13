use crate::domain::agent::AgentProfile;
use std::future::Future;

/// Catálogo de agentes verificados que se pueden instalar.
pub trait AgentCatalog: Send + Sync {
    /// Lista los agentes verificados, marcando si ya están instalados.
    fn list(&self) -> impl Future<Output = Result<Vec<AgentProfile>, String>> + Send;
}
