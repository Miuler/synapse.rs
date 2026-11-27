//! Registro de proveedores de issues: elige la conexión de un tablero y construye su adaptador.

use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::connection;
use crate::domain::issue::connection_repository::ConnectionRepository;
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind};
use crate::domain::issue::provider_descriptor::ProviderDescriptor;
use crate::infrastructure::repository::file::connection_repository_impl::ConnectionRepositoryImpl;
use crate::infrastructure::service::http::azure_devops_client::{self, AzureDevOpsClient};
use std::path::Path;

/// Los proveedores que admite esta versión.
pub fn descriptors() -> Vec<ProviderDescriptor> {
    vec![AzureDevOpsClient::descriptor()]
}

/// El proveedor de un tablero, con la configuración de este momento. `connection` es la que pide el
/// tablero; sin ella, la que cubra `scope`.
pub fn connect(
    provider: &str,
    connection: Option<&str>,
    scope: &str,
    config_dir: &Path,
) -> Result<Box<dyn IssueProvider>, IssueError> {
    if provider != azure_devops_client::PROVIDER {
        return Err(IssueError::new(IssueErrorKind::UnknownProvider, provider));
    }
    let connections = ConnectionRepositoryImpl::new(config_dir.into()).list()?;
    let chosen = connection::choose(&connections, provider, connection, scope)?;
    Ok(Box::new(AzureDevOpsClient::configured(config_dir, chosen)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_proveedor_registrado_se_puede_conectar() {
        let dir = tempfile::tempdir().unwrap();
        for descriptor in descriptors() {
            let provider = connect(&descriptor.id, None, "", dir.path()).ok().unwrap();
            assert_eq!(provider.describe(), descriptor);
        }
        let error = connect("otro", None, "", dir.path()).err().unwrap();
        assert_eq!(error.kind, IssueErrorKind::UnknownProvider);
        let error = connect("azure_devops", Some("borrada"), "", dir.path())
            .err()
            .unwrap();
        assert_eq!(error.kind, IssueErrorKind::NoConnection);
    }
}
