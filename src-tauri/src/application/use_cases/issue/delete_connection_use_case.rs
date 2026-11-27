use crate::application::service::credential_store::CredentialStore;
use crate::domain::issue::connection_repository::ConnectionRepository;
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind as Kind};

/// Quita una conexión y su token.
pub struct DeleteConnectionUseCase<R: ConnectionRepository, C: CredentialStore> {
    connections: R,
    credentials: C,
}

impl<R: ConnectionRepository, C: CredentialStore> DeleteConnectionUseCase<R, C> {
    pub fn new(connections: R, credentials: C) -> Self {
        Self {
            connections,
            credentials,
        }
    }

    pub fn execute(&self, id: &str) -> Result<(), IssueError> {
        let mut all = self.connections.list()?;
        let index = all
            .iter()
            .position(|c| c.id == id)
            .ok_or_else(|| IssueError::new(Kind::NoConnection, id))?;
        let removed = all.remove(index);
        self.credentials
            .delete(&removed.keyring_name())
            .map_err(|e| IssueError::new(Kind::Keyring, e))?;
        self.connections.save(&all)
    }
}
