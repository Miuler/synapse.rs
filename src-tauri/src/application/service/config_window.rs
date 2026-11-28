use crate::application::dto::issue::source_draft::{DraftChange, SourceDraft};

/// La configuración que el usuario tiene abierta en la ventana de Synapse. Solo se puede leer y
/// proponerle cambios: guardar es cosa del usuario.
pub trait ConfigWindow: Send + Sync {
    /// El borrador abierto; `None` si no hay ninguna configuración abierta.
    fn current(&self) -> Option<SourceDraft>;
    /// Pide a la ventana que cambie el borrador y devuelve cómo quedó.
    fn propose(&self, change: &DraftChange) -> Result<SourceDraft, String>;
}
