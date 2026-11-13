use serde::{Deserialize, Serialize};

/// Una parte del mensaje que se envía al agente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptBlock {
    Text {
        text: String,
    },
    /// Un archivo que el agente puede leer por su cuenta.
    File {
        path: String,
        name: String,
    },
    /// Un fragmento de un archivo, con sus líneas (1-based, inclusivas).
    Selection {
        path: String,
        text: String,
        start_line: u32,
        end_line: u32,
    },
    /// Una imagen en base64.
    Image {
        mime_type: String,
        data: String,
    },
}

impl PromptBlock {
    pub fn is_empty(&self) -> bool {
        match self {
            PromptBlock::Text { text } => text.trim().is_empty(),
            PromptBlock::Selection { text, .. } => text.is_empty(),
            PromptBlock::Image { data, .. } => data.is_empty(),
            PromptBlock::File { path, .. } => path.is_empty(),
        }
    }
}
