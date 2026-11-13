use crate::domain::agent::{
    AgentLaunch, AgentSession, PermissionDecision, PromptBlock, ToolServer,
};
use std::future::Future;

/// Ejecuta agentes externos y conversa con ellos. Las respuestas en curso, las herramientas
/// y las peticiones de permiso se notifican aparte mientras dura el prompt.
pub trait AgentRunner: Send + Sync {
    /// Lanza el agente y abre una conversación con acceso a `tools`. `instructions` se envían al
    /// agente una sola vez, junto con el primer mensaje. Con `resume` retoma esa sesión si el agente
    /// sabe cargarla; si no, abre una nueva.
    fn start(
        &self,
        agent_id: &str,
        launch: AgentLaunch,
        cwd: &str,
        tools: Vec<ToolServer>,
        instructions: Option<String>,
        resume: Option<String>,
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

    fn stop(&self, session_id: &str) -> impl Future<Output = Result<(), String>> + Send;
}
