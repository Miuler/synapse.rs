//! Agentes verificados que se usan desde su CLI oficial ya instalado, sin descarga.

use crate::domain::agent::AgentLaunch;
use std::path::PathBuf;

pub(super) struct LocalCliAgent {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub binary: &'static str,
    pub args: &'static [&'static str],
    /// Cómo instalarlo, para el mensaje cuando falta.
    pub install_hint: &'static str,
}

pub(super) const LOCAL_CLI_AGENTS: &[LocalCliAgent] = &[
    LocalCliAgent {
        id: "antigravity",
        name: "Google Antigravity",
        description: "Antigravity CLI ACP server (agy acp)",
        binary: "agy",
        args: &["acp"],
        install_hint: "Install the Antigravity CLI: curl -fsSL https://antigravity.google/cli/install.sh | bash",
    },
    LocalCliAgent {
        id: "github-copilot",
        name: "GitHub Copilot",
        description: "GitHub Copilot CLI ACP server (copilot --acp)",
        binary: "copilot",
        args: &["--acp"],
        install_hint: "Install the GitHub Copilot CLI: npm install -g @github/copilot (then sign in with copilot /login)",
    },
];

pub(super) fn find(id: &str) -> Option<&'static LocalCliAgent> {
    LOCAL_CLI_AGENTS.iter().find(|a| a.id == id)
}

impl LocalCliAgent {
    /// Ruta del ejecutable: el PATH y, como las apps gráficas a veces no lo heredan, `~/.local/bin`.
    pub fn locate(&self) -> Option<PathBuf> {
        let exe = if cfg!(windows) {
            format!("{}.exe", self.binary)
        } else {
            self.binary.to_string()
        };
        let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join(".local/bin"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("agy").join("bin"));
        }
        dirs.into_iter().map(|d| d.join(&exe)).find(|p| p.is_file())
    }

    pub fn launch(&self) -> Result<AgentLaunch, String> {
        let path = self
            .locate()
            .ok_or_else(|| format!("«{}» not found. {}", self.binary, self.install_hint))?;
        Ok(AgentLaunch {
            command: path.to_string_lossy().into_owned(),
            args: self.args.iter().map(|a| a.to_string()).collect(),
            env: Default::default(),
        })
    }
}
