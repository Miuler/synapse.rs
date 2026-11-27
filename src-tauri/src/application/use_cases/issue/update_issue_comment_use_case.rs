use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::issue_error::IssueError;

/// Cambia el texto de un comentario que ya está en el proveedor.
pub struct UpdateIssueCommentUseCase {
    provider: Box<dyn IssueProvider>,
}

impl UpdateIssueCommentUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(
        &self,
        scope: &str,
        id: &str,
        comment: &str,
        text: &str,
    ) -> Result<(), IssueError> {
        self.provider.update_comment(scope, id, comment, text).await
    }
}
