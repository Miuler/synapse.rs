use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::issue::Account;
use crate::domain::issue::issue_error::IssueError;

/// Prueba una conexión guardada.
pub struct CheckConnectionUseCase {
    provider: Box<dyn IssueProvider>,
}

impl CheckConnectionUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<Account, IssueError> {
        self.provider.check().await
    }
}
