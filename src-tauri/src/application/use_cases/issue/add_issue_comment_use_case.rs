use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::issue_error::IssueError;

/// Añade un comentario a una tarea.
pub struct AddIssueCommentUseCase {
    provider: Box<dyn IssueProvider>,
}

impl AddIssueCommentUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, scope: &str, id: &str, text: &str) -> Result<Option<String>, IssueError> {
        self.provider.add_comment(scope, id, text).await
    }
}
