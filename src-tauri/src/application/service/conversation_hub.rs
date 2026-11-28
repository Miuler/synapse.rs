use serde_json::Value;

/// Las conversaciones con agentes abiertas en la ventana de Synapse. Permite que un agente vea a
/// los demás y les deje mensajes; la entrega (y la cola si el destino está ocupado) la hace la ventana.
pub trait ConversationHub: Send + Sync {
    /// Conversaciones abiertas, marcando cuál es la de `caller`.
    fn list(&self, caller: Option<&str>) -> Result<Value, String>;
    /// Deja `message` en la conversación `to` (`#2`, su id o parte de su título). Devuelve si se
    /// entregó o quedó en cola; con `expects_reply`, la respuesta vuelve sola a `caller`.
    fn send(
        &self,
        caller: Option<&str>,
        to: &str,
        message: &str,
        expects_reply: bool,
    ) -> Result<Value, String>;
    /// Pedido de otra instancia de Synapse por el canal `/peer` (listar sus conversaciones,
    /// entregar un mensaje o una respuesta). Lo resuelve la ventana.
    /// Respuestas pendientes para un cliente MCP externo (se entregan una sola vez).
    fn replies(&self) -> Result<Value, String> {
        Err("This instance does not store replies for external clients".into())
    }
    fn peer(&self, _request: Value) -> Result<Value, String> {
        Err("This instance does not handle conversations from other vaults".into())
    }
}
