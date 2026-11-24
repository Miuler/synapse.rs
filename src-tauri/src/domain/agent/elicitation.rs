use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Respuesta del usuario a una solicitud de elicitación ACP (`elicitation/create`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum ElicitationDecision {
    Accept {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<Value>,
    },
    Decline,
    Cancel,
}
