//! Registro de proveedores de issues. Hasta que haya conexiones configurables, cada proveedor tiene
//! una implícita con su mismo identificador.

use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind};
use crate::domain::issue::provider_descriptor::ProviderDescriptor;
use crate::infrastructure::service::http::azure_devops_client::{self, AzureDevOpsClient};
use std::path::Path;

/// Los proveedores que admite esta versión.
pub fn descriptors() -> Vec<ProviderDescriptor> {
    vec![AzureDevOpsClient::descriptor()]
}

/// El proveedor de una conexión, con la configuración de este momento.
pub fn connect(connection: &str, config_dir: &Path) -> Result<Box<dyn IssueProvider>, IssueError> {
    match connection {
        azure_devops_client::PROVIDER => Ok(Box::new(AzureDevOpsClient::configured(config_dir))),
        other => Err(IssueError::new(IssueErrorKind::UnknownProvider, other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_proveedor_registrado_se_puede_conectar() {
        let dir = tempfile::tempdir().unwrap();
        for descriptor in descriptors() {
            let provider = connect(&descriptor.id, dir.path()).ok().unwrap();
            assert_eq!(provider.describe(), descriptor);
        }
        let error = connect("otro", dir.path()).err().unwrap();
        assert_eq!(error.kind, IssueErrorKind::UnknownProvider);
    }
}
