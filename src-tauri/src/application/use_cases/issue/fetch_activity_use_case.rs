use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::issue::IssueEntry;
use crate::domain::issue::issue_error::IssueError;
use futures_util::stream::{self, StreamExt};
use serde::Serialize;
use std::collections::BTreeMap;

/// Cuántas tareas se piden a la vez: un tablero de equipo tiene cientos y el proveedor limita.
const AT_ONCE: usize = 6;

#[derive(Debug, Serialize)]
pub struct ActivityFailure {
    pub id: String,
    pub error: IssueError,
}

#[derive(Debug, Default, Serialize)]
pub struct FetchedActivity {
    /// Por id de tarea.
    pub items: BTreeMap<String, Vec<IssueEntry>>,
    /// Las que no se pudieron leer: se vuelven a pedir otra vez.
    pub failed: Vec<ActivityFailure>,
}

/// La actividad de las tareas pedidas; que una falle no impide las demás.
pub struct FetchActivityUseCase {
    provider: Box<dyn IssueProvider>,
}

impl FetchActivityUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, scope: &str, ids: &[String]) -> FetchedActivity {
        let pending: Vec<_> = ids
            .iter()
            .map(|id| {
                let activity = self.provider.activity(scope, id);
                Box::pin(async move { (id.clone(), activity.await) })
            })
            .collect();
        let mut done = stream::iter(pending).buffer_unordered(AT_ONCE);
        let mut found = FetchedActivity::default();
        while let Some((id, result)) = done.next().await {
            match result {
                Ok(entries) => {
                    found.items.insert(id, entries);
                }
                Err(error) => found.failed.push(ActivityFailure { id, error }),
            }
        }
        found.failed.sort_by(|a, b| a.id.cmp(&b.id));
        found
    }
}
