use crate::domain::models::mcp::{McpPortRegistry, McpVaultConfig};
use std::path::Path;

/// Puerto/Trait de dominio para la persistencia y lectura de la configuración MCP por bóveda y global.
pub trait McpConfigRepository: Send + Sync {
    /// Lee la configuración específica de MCP de la bóveda (.synapse/mcp.json)
    fn read_vault_config(&self, vault_path: &Path) -> Option<McpVaultConfig>;

    /// Escribe la configuración específica de MCP de la bóveda (.synapse/mcp.json)
    fn write_vault_config(&self, vault_path: &Path, config: &McpVaultConfig) -> Result<(), String>;

    /// Comprueba si MCP debe estar activo para esta bóveda
    fn is_vault_enabled(&self, vault_path: &Path) -> bool;

    /// Obtiene el puerto asignado para la bóveda
    fn get_vault_port(&self, vault_path: &Path) -> Option<u16>;

    /// Lee el registro global de puertos MCP
    fn read_global_registry(&self) -> McpPortRegistry;

    /// Escribe el registro global de puertos MCP
    fn write_global_registry(&self, registry: &McpPortRegistry) -> Result<(), String>;

    /// Registra el inicio de MCP para una bóveda tanto en su .synapse/mcp.json como en el registro global
    fn record_mcp_started(&self, vault_path: &Path, port: u16);

    /// Registra la desactivación voluntaria de MCP por el usuario (marca enabled: false en la bóveda)
    fn record_mcp_disabled(&self, vault_path: &Path, port: u16);

    /// Registra el apagado técnico del proceso/socket efímero sin alterar la preferencia del vault
    fn record_mcp_runtime_stopped(&self, vault_path: &Path);
}
