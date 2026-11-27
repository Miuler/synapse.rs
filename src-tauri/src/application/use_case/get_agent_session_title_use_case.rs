use crate::application::dto::agent::GetAgentSessionTitleQuery;
use crate::application::integration::service::AgentRunner;

/// Nombre que el agente tiene para una sesión abierta.
#[derive(Clone)]
pub struct GetAgentSessionTitleUseCase<A: AgentRunner> {
    runner: A,
}

impl<A: AgentRunner> GetAgentSessionTitleUseCase<A> {
    pub fn new(runner: A) -> Self {
        Self { runner }
    }

    pub async fn execute(
        &self,
        query: GetAgentSessionTitleQuery,
    ) -> Result<Option<String>, String> {
        self.runner.title(&query.session_id).await
    }
}
