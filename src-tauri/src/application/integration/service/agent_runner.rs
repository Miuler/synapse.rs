use crate::domain::agent::{
    AgentConfig, AgentLaunch, AgentSession, ElicitationDecision, PermissionDecision, PromptBlock, ToolServer,
};
use std::future::Future;

/// Ejecuta agentes externos y conversa con ellos. Las respuestas en curso, las herramientas
/// y las peticiones de permiso se notifican aparte mientras dura el prompt.
pub trait AgentRunner: Send + Sync {
    /// Lanza el agente y abre una conversación con acceso a `tools`. `instructions` se envían al
    /// agente una sola vez, junto con el primer mensaje. Con `resume` retoma esa sesión si el agente
    /// sabe cargarla; si no, abre una nueva.
    #[allow(clippy::too_many_arguments)]
    fn start(
        &self,
        agent_id: &str,
        launch: AgentLaunch,
        cwd: &str,
        tools: Vec<ToolServer>,
        instructions: Option<String>,
        resume: Option<String>,
        fork: bool,
    ) -> impl Future<Output = Result<AgentSession, String>> + Send;

    /// Envía un mensaje y espera a que el agente termine el turno; devuelve el motivo de parada.
    fn prompt(
        &self,
        session_id: &str,
        blocks: Vec<PromptBlock>,
    ) -> impl Future<Output = Result<String, String>> + Send;

    /// Cancela el turno en curso de la sesión.
    fn cancel(&self, session_id: &str) -> impl Future<Output = Result<(), String>> + Send;

    fn respond_permission(
        &self,
        session_id: &str,
        request_id: &str,
        decision: PermissionDecision,
    ) -> impl Future<Output = Result<(), String>> + Send;

    fn respond_elicitation(
        &self,
        session_id: &str,
        request_id: &str,
        decision: ElicitationDecision,
    ) -> impl Future<Output = Result<(), String>> + Send;

    fn set_mode(
        &self,
        session_id: &str,
        mode_id: &str,
    ) -> impl Future<Output = Result<(), String>> + Send;

    fn set_model(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> impl Future<Output = Result<(), String>> + Send;

    /// Cambia una opción de configuración y devuelve el modelo y el esfuerzo resultantes.
    fn set_option(
        &self,
        session_id: &str,
        option_id: &str,
        value: &str,
    ) -> impl Future<Output = Result<AgentConfig, String>> + Send;

    fn stop(&self, session_id: &str) -> impl Future<Output = Result<(), String>> + Send;

    /// Conversaciones que comparte un mismo proceso de `agent_id` (con más, se lanza otro).
    fn set_process_limit(&self, agent_id: &str, limit: usize);
}
