//! Agentes externos por ACP (Agent Client Protocol): JSON-RPC por `stdio`.
//!
//! Los agentes verificados salen del registro público de ACP, el mismo que usa Zed:
//! <https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json>.

pub mod agent_catalog_impl;
pub mod agent_installer_impl;
pub mod agent_runner_impl;
mod connection;
pub mod debug_log;
mod local_cli;
pub mod registry;

pub use agent_catalog_impl::AgentCatalogImpl;
pub use agent_installer_impl::AgentInstallerImpl;
pub use agent_runner_impl::{AgentEventSink, AgentRunnerImpl};
