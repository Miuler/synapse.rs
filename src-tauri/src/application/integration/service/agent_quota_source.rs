use crate::domain::agent::AgentQuota;
use std::future::Future;

/// Cuota de la cuenta, para los agentes que no la informan por su cuenta durante la conversación.
pub trait AgentQuotaSource: Send + Sync {
    /// `None` si de este agente no se puede saber.
    fn quota(
        &self,
        agent_id: &str,
        max_age_ms: i64,
    ) -> impl Future<Output = Result<Option<AgentQuota>, String>> + Send;
}
