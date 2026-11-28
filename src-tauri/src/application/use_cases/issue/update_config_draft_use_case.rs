use crate::application::dto::issue::source_draft::{DraftChange, SourceDraft};
use crate::application::service::config_window::ConfigWindow;
use std::sync::Arc;

/// Cambia el borrador abierto. No guarda: lo decide el usuario.
pub struct UpdateConfigDraftUseCase {
    window: Arc<dyn ConfigWindow>,
}

impl UpdateConfigDraftUseCase {
    pub fn new(window: Arc<dyn ConfigWindow>) -> Self {
        Self { window }
    }

    pub fn execute(
        &self,
        draft: &SourceDraft,
        change: &DraftChange,
    ) -> Result<SourceDraft, String> {
        if *change == DraftChange::default() {
            return Err("nothing to update: provide `query`, `filter`, or `states`".into());
        }
        if change.query.is_some() && change.filter.is_some() {
            return Err("`query` and `filter` are mutually exclusive: provide only one".into());
        }
        for (column, states) in change.states.iter().flatten() {
            if !draft.columns.iter().any(|c| &c.id == column) {
                let known: Vec<&str> = draft.columns.iter().map(|c| c.id.as_str()).collect();
                return Err(format!(
                    "Board does not have column «{column}»; available: {}",
                    known.join(", ")
                ));
            }
            if states.iter().any(|state| state.trim().is_empty()) {
                return Err(format!("{column}: column has an empty state"));
            }
        }
        self.window.propose(change)
    }
}
