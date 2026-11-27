use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::issue::{Issue, NewIssue};
use crate::domain::issue::issue_error::IssueError;

/// Crea una tarea.
pub struct CreateIssueUseCase {
    provider: Box<dyn IssueProvider>,
}

impl CreateIssueUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, scope: &str, issue: &NewIssue) -> Result<Issue, IssueError> {
        self.provider.create(scope, issue).await
    }
}
