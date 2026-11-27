use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::issue::{Issue, IssueChanges};
use crate::domain::issue::issue_error::IssueError;

/// Cambia una tarea sobre la revisión que se leyó.
pub struct UpdateIssueUseCase {
    provider: Box<dyn IssueProvider>,
}

impl UpdateIssueUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(
        &self,
        scope: &str,
        id: &str,
        revision: Option<&str>,
        changes: &IssueChanges,
    ) -> Result<Issue, IssueError> {
        self.provider.update(scope, id, revision, changes).await
    }
}
