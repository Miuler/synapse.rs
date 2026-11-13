use crate::domain::agent::{AgentProfile, AgentProfileRepository};
use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Agentes personalizados en `~/.config/synapsers/agents.json`.
#[derive(Clone)]
pub struct AgentProfileRepositoryImpl {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl Default for AgentProfileRepositoryImpl {
    fn default() -> Self {
        Self::new(WorkspaceRegistryRepositoryImpl::resolve_config_dir().join("agents.json"))
    }
}

impl AgentProfileRepositoryImpl {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Arc::default(),
        }
    }

    fn read(&self) -> Result<Vec<AgentProfile>, String> {
        match std::fs::read(&self.path) {
            Ok(data) => {
                serde_json::from_slice(&data).map_err(|e| format!("agents.json inválido: {e}"))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.to_string()),
        }
    }

    fn write(&self, profiles: &[AgentProfile]) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_vec_pretty(profiles).map_err(|e| e.to_string())?;
        // Archivo temporal + renombrado para no dejarlo a medias
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

impl AgentProfileRepository for AgentProfileRepositoryImpl {
    fn list(&self) -> Result<Vec<AgentProfile>, String> {
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        self.read()
    }

    fn save(&self, profile: AgentProfile) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        let mut all = self.read()?;
        match all.iter_mut().find(|p| p.id == profile.id) {
            Some(existing) => *existing = profile,
            None => all.push(profile),
        }
        self.write(&all)
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        let mut all = self.read()?;
        all.retain(|p| p.id != id);
        self.write(&all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent::AgentLaunch;

    #[test]
    fn guarda_edita_y_borra() {
        let dir = tempfile::tempdir().unwrap();
        let repo = AgentProfileRepositoryImpl::new(dir.path().join("agents.json"));
        assert!(repo.list().unwrap().is_empty());
        let launch = AgentLaunch {
            command: "a".into(),
            ..Default::default()
        };
        repo.save(AgentProfile::custom("uno", "Uno", launch.clone()).unwrap())
            .unwrap();
        repo.save(AgentProfile::custom("uno", "Uno bis", launch).unwrap())
            .unwrap();
        let all = repo.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Uno bis");
        repo.delete("uno").unwrap();
        assert!(repo.list().unwrap().is_empty());
    }
}
