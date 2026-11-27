use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::filter::{Criterion, IssueFilter};
use crate::domain::issue::issue_error::IssueError;

/// Los valores de un criterio y la consulta que corresponde a un filtro.
pub struct FilterIssuesUseCase {
    provider: Box<dyn IssueProvider>,
}

impl FilterIssuesUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn values(
        &self,
        scope: &str,
        criterion: Criterion,
    ) -> Result<Vec<String>, IssueError> {
        self.provider.values(scope, criterion).await
    }

    pub async fn query(
        &self,
        scope: &str,
        filter: &IssueFilter,
        today: &str,
    ) -> Result<String, IssueError> {
        self.provider.filter_query(scope, filter, today).await
    }
}
