//! Conversaciones entre agentes por el MCP: `list_conversations` y `send_to_conversation`.
use app_lib::application::service::conversation_hub::ConversationHub;
use app_lib::domain::search::full_text_index::FullTextIndex;
use app_lib::infrastructure::entrypoint::mcp::tool_handler::McpToolHandler;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use app_lib::infrastructure::repository::file::pdf_document_repository_impl::PdfDocumentRepositoryImpl;
use app_lib::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use app_lib::infrastructure::repository::git::version_control_impl::VersionControlImpl;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct InMemoryConversationHubImpl {
    sent: Mutex<Vec<Value>>,
}

impl ConversationHub for InMemoryConversationHubImpl {
    fn list(&self, caller: Option<&str>) -> Result<Value, String> {
        Ok(
            json!([{"handle": "#1", "is_you": caller == Some("a")}, {"handle": "#2", "is_you": false}]),
        )
    }
    fn send(
        &self,
        caller: Option<&str>,
        to: &str,
        message: &str,
        expects_reply: bool,
    ) -> Result<Value, String> {
        self.sent.lock().unwrap().push(
            json!({"caller": caller, "to": to, "message": message, "expects_reply": expects_reply}),
        );
        Ok(json!({"status": "queued"}))
    }
    fn replies(&self) -> Result<Value, String> {
        Ok(json!([{"from": "#1 · Claude", "text": "sí"}]))
    }
}

type Handler = McpToolHandler<NoteRepositoryImpl, dyn FullTextIndex, VersionControlImpl>;

fn handler(hub: Option<Arc<InMemoryConversationHubImpl>>, caller: Option<&str>) -> Handler {
    McpToolHandler::new(
        std::env::temp_dir(),
        NoteRepositoryImpl::new(),
        None::<Arc<dyn FullTextIndex>>,
        None::<Arc<VersionControlImpl>>,
        Arc::new(VaultItemRepositoryImpl),
        Arc::new(PdfDocumentRepositoryImpl),
    )
    .with_conversations(
        hub.map(|h| h as Arc<dyn ConversationHub>),
        caller.map(str::to_string),
    )
}

fn call(h: &Handler, name: &str, args: Value) -> Value {
    serde_json::to_value(h.call_tool(name, Some(args))).unwrap()
}

#[test]
fn lista_y_envia_indicando_quien_llama() {
    let hub = Arc::new(InMemoryConversationHubImpl::default());
    let h = handler(Some(hub.clone()), Some("a"));
    let list = call(&h, "list_conversations", json!({}));
    let parsed: Value = serde_json::from_str(list["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(parsed[0]["is_you"], true);

    let sent = call(
        &h,
        "send_to_conversation",
        json!({"to": "#2", "message": "¿tiene SignOn?", "expects_reply": true}),
    );
    assert_eq!(sent["isError"], false);
    let rec = hub.sent.lock().unwrap()[0].clone();
    assert_eq!(
        rec,
        json!({"caller": "a", "to": "#2", "message": "¿tiene SignOn?", "expects_reply": true})
    );

    let bad = call(&h, "send_to_conversation", json!({"to": "#2"}));
    assert_eq!(bad["isError"], true);
}

#[test]
fn sin_ventana_explica_que_falta() {
    let h = handler(None, None);
    let r = call(&h, "list_conversations", json!({}));
    assert_eq!(r["isError"], true);
    assert!(
        r["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("window")
    );
}

#[test]
fn un_cliente_externo_escribe_y_recoge_respuestas() {
    let hub = Arc::new(InMemoryConversationHubImpl::default());
    // Sin conversación propia (sin `?conv=`)
    let h = handler(Some(hub.clone()), None);
    let sent = call(
        &h,
        "send_to_conversation",
        json!({"to": "#1", "message": "¿hola?", "expects_reply": true}),
    );
    assert_eq!(sent["isError"], false);
    assert_eq!(hub.sent.lock().unwrap()[0]["caller"], Value::Null);
    let replies = call(&h, "read_replies", json!({}));
    assert!(
        replies["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("\"sí\"")
    );
}
