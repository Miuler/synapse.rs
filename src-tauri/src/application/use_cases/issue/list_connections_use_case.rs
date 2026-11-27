use crate::application::dto::issue::connection_view::ConnectionView;
use crate::application::service::credential_store::CredentialStore;
use crate::domain::issue::connection_repository::ConnectionRepository;
use crate::domain::issue::issue_error::IssueError;

pub struct ListConnectionsUseCase<R: ConnectionRepository, C: CredentialStore> {
    connections: R,
    credentials: C,
}

impl<R: ConnectionRepository, C: CredentialStore> ListConnectionsUseCase<R, C> {
    pub fn new(connections: R, credentials: C) -> Self {
        Self {
            connections,
            credentials,
        }
    }

    pub fn execute(&self) -> Result<Vec<ConnectionView>, IssueError> {
        Ok(self
            .connections
            .list()?
            .into_iter()
            .map(|c| {
                // Con referencia, el llavero no se consulta: podría pedir que se desbloquee. Si no
                // responde, la conexión se muestra igual
                let stored = c.secret.is_none()
                    && matches!(self.credentials.get(&c.keyring_name()), Ok(Some(_)));
                ConnectionView::new(c, stored)
            })
            .collect())
    }
}
