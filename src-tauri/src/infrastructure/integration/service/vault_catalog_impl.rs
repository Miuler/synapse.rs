use crate::application::service::vault_catalog::{VaultCatalog, VaultRef};
use crate::domain::workspace::workspace_registry::WorkspaceRegistryConfig;
use crate::infrastructure::entrypoint::mcp::port_registry;
use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use std::path::PathBuf;

/// Bóvedas recientes (`workspaces.json`) y las abiertas ahora (registro de puertos del MCP).
#[derive(Clone, Default)]
pub struct VaultCatalogImpl;

impl VaultCatalog for VaultCatalogImpl {
    fn list(&self) -> Vec<VaultRef> {
        let open: Vec<PathBuf> = port_registry::read_global_registry()
            .vaults
            .values()
            .filter(|e| e.running)
            .map(|e| PathBuf::from(&e.vault_path))
            .collect();
        let recents: WorkspaceRegistryConfig = std::fs::read_to_string(
            WorkspaceRegistryRepositoryImpl::resolve_config_dir().join("workspaces.json"),
        )
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
        let mut out: Vec<VaultRef> = Vec::new();
        let paths = recents
            .workspaces
            .iter()
            .map(|w| PathBuf::from(&w.path))
            .chain(open.iter().cloned());
        for path in paths {
            let path = path.canonicalize().unwrap_or(path);
            if !path.is_dir() || out.iter().any(|v| v.path == path) {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            let is_open = open
                .iter()
                .any(|o| o.canonicalize().unwrap_or_else(|_| o.clone()) == path);
            out.push(VaultRef {
                name,
                path,
                open: is_open,
            });
        }
        out
    }
}
