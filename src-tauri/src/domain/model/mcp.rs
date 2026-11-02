use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpVaultConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default)]
    pub last_endpoint: Option<String>,
}

fn default_mcp_port() -> u16 {
    27182
}

impl Default for McpVaultConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_mcp_port(),
            last_endpoint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpPortRegistryEntry {
    pub vault_path: String,
    pub port: u16,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct McpPortRegistry {
    #[serde(default)]
    pub vaults: HashMap<String, McpPortRegistryEntry>,
}
