use crate::application::dto::agent::ListAgentsQuery;
use crate::application::integration::service::AgentCatalog;
use crate::domain::agent::{AgentProfile, AgentProfileRepository};

/// Lista los agentes verificados del catálogo y los personalizados del usuario.
#[derive(Clone)]
pub struct ListAgentsUseCase<C: AgentCatalog, R: AgentProfileRepository> {
    catalog: C,
    profiles: R,
}

impl<C: AgentCatalog, R: AgentProfileRepository> ListAgentsUseCase<C, R> {
    pub fn new(catalog: C, profiles: R) -> Self {
        Self { catalog, profiles }
    }

    pub async fn execute(&self, _query: ListAgentsQuery) -> Result<Vec<AgentProfile>, String> {
        // Sin red el catálogo puede fallar; los personalizados se siguen mostrando
        let mut agents = self.catalog.list().await.unwrap_or_else(|e| {
            tracing::warn!("No se pudo leer el catálogo de agentes: {e}");
            vec![]
        });
        let custom = self.profiles.list()?;
        // Un personalizado con el mismo id que uno verificado lo reemplaza
        agents.retain(|a| !custom.iter().any(|c| c.id == a.id));
        agents.extend(custom);
        Ok(agents)
    }
}
