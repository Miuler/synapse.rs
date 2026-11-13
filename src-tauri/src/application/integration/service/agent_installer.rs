use crate::domain::agent::AgentLaunch;
use std::future::Future;

/// Instala un agente verificado y dice cómo lanzarlo.
pub trait AgentInstaller: Send + Sync {
    /// Descarga e instala el agente si hace falta; devuelve cómo lanzarlo.
    fn ensure_installed(
        &self,
        agent_id: &str,
    ) -> impl Future<Output = Result<AgentLaunch, String>> + Send;
}
