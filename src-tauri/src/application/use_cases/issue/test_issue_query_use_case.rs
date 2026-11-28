use crate::application::dto::issue::source_draft::{QueryMode, SourceDraft};
use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::filter::IssueFilter;
use crate::domain::issue::issue_error::IssueError;
use serde::Serialize;

/// Tareas que se muestran como mucho al probar una consulta.
pub const MAX_ROWS: usize = 20;

#[derive(Debug, Serialize)]
pub struct QueryRow {
    #[serde(rename = "ref")]
    pub key: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub title: String,
    pub state: String,
    pub assignee: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryTest {
    /// La consulta que se ejecutó.
    pub query: String,
    pub count: usize,
    pub rows: Vec<QueryRow>,
}

/// Ejecuta en solo lectura una consulta o un filtro en el origen del borrador.
pub struct TestIssueQueryUseCase {
    provider: Box<dyn IssueProvider>,
}

impl TestIssueQueryUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    /// Sin `query` ni `filter`, prueba lo que haya en el borrador.
    pub async fn execute(
        &self,
        draft: &SourceDraft,
        query: Option<String>,
        filter: Option<IssueFilter>,
    ) -> Result<QueryTest, IssueError> {
        let scope = &draft.scope;
        let filter = filter.or_else(|| {
            (query.is_none() && draft.mode == QueryMode::Criteria).then(|| draft.filter.clone())
        });
        let query = match filter {
            Some(filter) => {
                self.provider
                    .filter_query(scope, &filter, &draft.today)
                    .await?
            }
            None => query
                .or_else(|| (draft.mode == QueryMode::Custom).then(|| draft.query.clone()))
                .filter(|q| !q.trim().is_empty())
                .unwrap_or_else(|| self.provider.describe().default_query),
        };
        let ids = self.provider.search(scope, &query).await?;
        let shown = &ids[..ids.len().min(MAX_ROWS)];
        let mut found = if shown.is_empty() {
            Vec::new()
        } else {
            self.provider.get(scope, shown).await?
        };
        // En el orden en que las devolvió la consulta
        found.sort_by_key(|issue| shown.iter().position(|id| *id == issue.id));
        Ok(QueryTest {
            query,
            count: ids.len(),
            rows: found
                .into_iter()
                .map(|issue| QueryRow {
                    key: issue.key,
                    issue_type: issue.issue_type,
                    title: issue.title,
                    state: issue.state.name,
                    assignee: issue.assignee.map(|person| person.name),
                })
                .collect(),
        })
    }
}
