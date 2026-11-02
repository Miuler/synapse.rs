use crate::domain::mcp::mcp::{McpPortRegistry, McpVaultConfig};
use crate::domain::mcp::mcp_repository::McpConfigRepository;
use crate::infrastructure::entrypoint::mcp::port_registry;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct McpConfigRepositoryImpl;

impl McpConfigRepositoryImpl {
    pub fn new() -> Self {
        Self
    }
}

impl McpConfigRepository for McpConfigRepositoryImpl {
    fn read_vault_config(&self, vault_path: &Path) -> Option<McpVaultConfig> {
        port_registry::read_vault_config(vault_path)
    }

    fn write_vault_config(&self, vault_path: &Path, config: &McpVaultConfig) -> Result<(), String> {
        port_registry::write_vault_config(vault_path, config);
        Ok(())
    }

    fn is_vault_enabled(&self, vault_path: &Path) -> bool {
        port_registry::is_vault_enabled(vault_path)
    }

    fn get_vault_port(&self, vault_path: &Path) -> Option<u16> {
        port_registry::get_vault_port(vault_path)
    }

    fn read_global_registry(&self) -> McpPortRegistry {
        port_registry::read_global_registry()
    }

    fn write_global_registry(&self, registry: &McpPortRegistry) -> Result<(), String> {
        port_registry::write_global_registry(registry);
        Ok(())
    }

    fn record_mcp_started(&self, vault_path: &Path, port: u16) {
        port_registry::record_mcp_started(vault_path, port);
    }

    fn record_mcp_disabled(&self, vault_path: &Path, port: u16) {
        port_registry::record_mcp_disabled(vault_path, port);
    }

    fn record_mcp_runtime_stopped(&self, vault_path: &Path) {
        port_registry::record_mcp_runtime_stopped(vault_path);
    }
}
