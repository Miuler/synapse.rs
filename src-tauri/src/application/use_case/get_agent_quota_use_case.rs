use crate::application::dto::agent::GetAgentQuotaQuery;
use crate::application::integration::service::AgentQuotaSource;
use crate::domain::agent::{AgentProfileRepository, AgentQuota};

/// Cuota de la cuenta de un agente verificado. La de uno personalizado no se conoce: con el mismo id
/// puede lanzar otro programa o usar otra cuenta.
#[derive(Clone)]
pub struct GetAgentQuotaUseCase<R: AgentProfileRepository, Q: AgentQuotaSource> {
    profiles: R,
    source: Q,
}

impl<R: AgentProfileRepository, Q: AgentQuotaSource> GetAgentQuotaUseCase<R, Q> {
    pub fn new(profiles: R, source: Q) -> Self {
        Self { profiles, source }
    }

    pub async fn execute(&self, query: GetAgentQuotaQuery) -> Result<Option<AgentQuota>, String> {
        if self.profiles.list()?.iter().any(|p| p.id == query.agent_id) {
            return Ok(None);
        }
        self.source.quota(&query.agent_id, query.max_age_ms).await
    }
}
