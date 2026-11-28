use crate::application::dto::issue::source_draft::SourceDraft;
use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::filter::Criterion;
use crate::domain::issue::issue_error::IssueError;

/// Valores que se entregan como mucho de cada criterio.
pub const MAX_VALUES: usize = 200;
/// Tareas vinculadas que se leen para saber a quién están asignadas.
const MAX_LINKED: usize = 200;

pub struct CriterionValues {
    pub criterion: Criterion,
    /// Los primeros `MAX_VALUES` y cuántos hay; el error es solo de este criterio.
    pub values: Result<(Vec<String>, usize), IssueError>,
}

/// Los valores que admite el origen del borrador, criterio por criterio.
pub struct DescribeSourceUseCase {
    provider: Box<dyn IssueProvider>,
}

impl DescribeSourceUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, draft: &SourceDraft, wanted: &[Criterion]) -> Vec<CriterionValues> {
        let mut found = Vec::new();
        for &criterion in wanted {
            let values = self.values(draft, criterion).await.map(|mut values| {
                let total = values.len();
                values.truncate(MAX_VALUES);
                (values, total)
            });
            found.push(CriterionValues { criterion, values });
        }
        found
    }

    async fn values(
        &self,
        draft: &SourceDraft,
        criterion: Criterion,
    ) -> Result<Vec<String>, IssueError> {
        let mut values = self.provider.values(&draft.scope, criterion).await?;
        // El proveedor no lista personas: se añaden las ya asignadas a las tareas vinculadas
        if criterion == Criterion::Assignee && !draft.linked.is_empty() {
            let linked = &draft.linked[..draft.linked.len().min(MAX_LINKED)];
            for issue in self.provider.get(&draft.scope, linked).await? {
                if let Some(person) = issue.assignee
                    && !person.id.is_empty()
                    && !values.contains(&person.id)
                {
                    values.push(person.id);
                }
            }
        }
        Ok(values)
    }
}
