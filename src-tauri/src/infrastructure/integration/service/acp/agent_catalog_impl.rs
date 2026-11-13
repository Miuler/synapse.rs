use super::local_cli::LOCAL_CLI_AGENTS;
use super::registry::{RegistryClient, VERIFIED_AGENTS};
use crate::application::integration::service::AgentCatalog;
use crate::domain::agent::{AgentProfile, AgentSource};

/// Catálogo verificado a partir del registro público de ACP.
#[derive(Clone)]
pub struct AgentCatalogImpl {
    registry: RegistryClient,
}

impl AgentCatalogImpl {
    pub fn new(registry: RegistryClient) -> Self {
        Self { registry }
    }
}

impl AgentCatalog for AgentCatalogImpl {
    async fn list(&self) -> Result<Vec<AgentProfile>, String> {
        let local = LOCAL_CLI_AGENTS.iter().map(|a| AgentProfile {
            id: a.id.to_string(),
            name: a.name.to_string(),
            source: AgentSource::Verified,
            description: Some(a.description.to_string()),
            version: None,
            installed: a.locate().is_some(),
            launch: None,
        });
        // Sin red, los agentes de CLI local siguen disponibles
        let registry = match self.registry.registry().await {
            Ok(r) => r,
            Err(e) => {
                let agents: Vec<_> = local.collect();
                return if agents.is_empty() {
                    Err(e)
                } else {
                    Ok(agents)
                };
            }
        };
        let mut agents: Vec<AgentProfile> = VERIFIED_AGENTS
            .iter()
            .filter_map(|id| registry.agents.iter().find(|a| a.id == *id))
            .map(|a| AgentProfile {
                id: a.id.clone(),
                name: a.name.clone(),
                source: AgentSource::Verified,
                description: a.description.clone(),
                version: Some(a.version.clone()),
                installed: a.installed_launch().is_some(),
                launch: None,
            })
            .collect();
        agents.extend(local);
        Ok(agents)
    }
}
