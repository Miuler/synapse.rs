//! Registro público de agentes ACP (el mismo que usa Zed) y carpeta donde se instalan.

use crate::domain::agent::AgentLaunch;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub(super) const REGISTRY_URL: &str =
    "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";

/// Agentes del registro que Synapse ofrece como verificados. Se agregan de a uno tras probarlos.
pub(super) const VERIFIED_AGENTS: &[&str] = &["claude-acp"];

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);
const INSTALLED_MARKER: &str = ".synapse-launch.json";

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Registry {
    pub agents: Vec<RegistryAgent>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RegistryAgent {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub distribution: Distribution,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct Distribution {
    #[serde(default)]
    pub binary: Option<HashMap<String, BinaryTarget>>,
    #[serde(default)]
    pub npx: Option<PackageTarget>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct BinaryTarget {
    pub archive: String,
    pub cmd: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct PackageTarget {
    pub package: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl RegistryAgent {
    /// Binario para esta plataforma, si el agente lo publica.
    pub fn binary_for_platform(&self) -> Option<&BinaryTarget> {
        self.distribution.binary.as_ref()?.get(platform_key()?)
    }

    /// Carpeta de instalación de esta versión.
    pub fn install_dir(&self) -> PathBuf {
        agents_dir().join(&self.id).join(&self.version)
    }

    /// Cómo lanzarlo si ya está instalado.
    pub fn installed_launch(&self) -> Option<AgentLaunch> {
        let data = std::fs::read(self.install_dir().join(INSTALLED_MARKER)).ok()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn mark_installed(&self, launch: &AgentLaunch) -> Result<(), String> {
        let data = serde_json::to_vec_pretty(launch).map_err(|e| e.to_string())?;
        std::fs::write(self.install_dir().join(INSTALLED_MARKER), data).map_err(|e| e.to_string())
    }
}

/// Clave de plataforma del registro (`linux-x86_64`, `darwin-aarch64`…).
pub(super) fn platform_key() -> Option<&'static str> {
    let musl = cfg!(target_env = "musl");
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") if musl => "linux-x86_64-musl",
        ("linux", "aarch64") if musl => "linux-aarch64-musl",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("macos", "aarch64") => "darwin-aarch64",
        ("windows", "x86_64") => "windows-x86_64",
        ("windows", "aarch64") => "windows-aarch64",
        _ => return None,
    })
}

/// Carpeta de datos donde Synapse guarda los agentes descargados.
pub(super) fn agents_dir() -> PathBuf {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let base = if cfg!(windows) {
        env("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    base.unwrap_or_else(std::env::temp_dir)
        .join("synapsers")
        .join("agents")
}

type CachedRegistry = (Instant, Arc<Registry>);

/// Descarga el registro con caché en memoria (1 h) y copia en disco para trabajar sin red.
#[derive(Clone)]
pub struct RegistryClient {
    http: reqwest::Client,
    cache: Arc<Mutex<Option<CachedRegistry>>>,
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("synapsers")
            .connect_timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self {
            http,
            cache: Arc::default(),
        }
    }

    pub(super) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub(super) async fn registry(&self) -> Result<Arc<Registry>, String> {
        let mut cache = self.cache.lock().await;
        if let Some((at, reg)) = cache.as_ref()
            && at.elapsed() < CACHE_TTL
        {
            return Ok(reg.clone());
        }
        let copy = agents_dir().join("registry.json");
        let fetched = async {
            let resp = self
                .http
                // SYNAPSE_ACP_REGISTRY_URL: las e2e la apuntan a un puerto cerrado para no depender de la red
                .get(std::env::var("SYNAPSE_ACP_REGISTRY_URL").unwrap_or_else(|_| REGISTRY_URL.to_string()))
                .timeout(Duration::from_secs(30))
                .send()
                .await?;
            resp.error_for_status()?.bytes().await
        }
        .await;
        let bytes = match fetched {
            Ok(bytes) => {
                let _ = std::fs::create_dir_all(agents_dir());
                let _ = std::fs::write(&copy, &bytes);
                bytes.to_vec()
            }
            Err(e) => std::fs::read(&copy)
                .map_err(|_| format!("Could not download agent registry: {e}"))?,
        };
        let reg: Registry = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Invalid agent registry: {e}"))?;
        let reg = Arc::new(reg);
        *cache = Some((Instant::now(), reg.clone()));
        Ok(reg)
    }

    /// Entrada verificada del registro.
    pub(super) async fn verified(&self, agent_id: &str) -> Result<RegistryAgent, String> {
        if !VERIFIED_AGENTS.contains(&agent_id) {
            return Err(format!("{agent_id}: not a verified agent"));
        }
        self.registry()
            .await?
            .agents
            .iter()
            .find(|a| a.id == agent_id)
            .cloned()
            .ok_or_else(|| format!("Agent registry no longer includes «{agent_id}»"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_una_entrada_del_registro() {
        let json = r#"{"version":"1.0.0","agents":[{"id":"claude-acp","name":"Claude Agent","version":"0.107.0",
            "description":"ACP wrapper","distribution":{
              "binary":{"linux-x86_64":{"archive":"https://x/claude.tar.gz","cmd":"./claude-agent-acp","sha256":"ab"}},
              "npx":{"package":"@agentclientprotocol/claude-agent-acp@0.107.0"}}}]}"#;
        let reg: Registry = serde_json::from_str(json).unwrap();
        let agent = &reg.agents[0];
        assert_eq!(
            agent.distribution.npx.as_ref().unwrap().package,
            "@agentclientprotocol/claude-agent-acp@0.107.0"
        );
        let bin = &agent.distribution.binary.as_ref().unwrap()["linux-x86_64"];
        assert_eq!(bin.cmd, "./claude-agent-acp");
        assert!(bin.args.is_empty());
    }
}
