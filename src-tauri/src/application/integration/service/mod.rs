//! Contratos de servicios externos: procesos y servicios que no controlamos.

pub mod agent_catalog;
pub mod agent_installer;
pub mod agent_quota_source;
pub mod agent_runner;

pub use agent_catalog::AgentCatalog;
pub use agent_installer::AgentInstaller;
pub use agent_quota_source::AgentQuotaSource;
pub use agent_runner::AgentRunner;
