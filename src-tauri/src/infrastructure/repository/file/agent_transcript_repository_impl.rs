use crate::domain::agent::{AgentTranscript, AgentTranscriptRepository, AgentTranscriptSummary};
use std::path::{Path, PathBuf};

/// Conversaciones dentro de la bóveda: `<bóveda>/.synapse/agent-sessions/<id>.json`, una por archivo.
#[derive(Clone, Default)]
pub struct AgentTranscriptRepositoryImpl;

impl AgentTranscriptRepositoryImpl {
    fn dir(cwd: &str) -> Result<PathBuf, String> {
        if cwd.is_empty() || !Path::new(cwd).is_dir() {
            return Err(format!("La bóveda «{cwd}» no existe"));
        }
        Ok(Path::new(cwd).join(".synapse").join("agent-sessions"))
    }

    fn file_name(id: &str) -> Result<String, String> {
        if AgentTranscript::valid_id(id) {
            Ok(format!("{id}.json"))
        } else {
            Err(format!("Identificador de conversación inválido: {id}"))
        }
    }

    fn files(cwd: &str) -> Result<Vec<PathBuf>, String> {
        match std::fs::read_dir(Self::dir(cwd)?) {
            Ok(entries) => Ok(entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl AgentTranscriptRepository for AgentTranscriptRepositoryImpl {
    fn save(&self, transcript: AgentTranscript) -> Result<(), String> {
        let dir = Self::dir(&transcript.cwd)?;
        let path = dir.join(Self::file_name(&transcript.id)?);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let data = serde_json::to_vec_pretty(&transcript).map_err(|e| e.to_string())?;
        // Archivo temporal + renombrado para no dejarlo a medias
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }

    fn list(&self, cwd: &str) -> Result<Vec<AgentTranscriptSummary>, String> {
        let mut all: Vec<AgentTranscriptSummary> = Self::files(cwd)?
            .into_iter()
            .filter_map(|p| std::fs::read(p).ok())
            .filter_map(|data| serde_json::from_slice::<AgentTranscript>(&data).ok())
            .map(|t| AgentTranscriptSummary::from(&t))
            .collect();
        all.sort_by_key(|t| std::cmp::Reverse(t.updated_at));
        Ok(all)
    }

    fn load(&self, cwd: &str, id: &str) -> Result<AgentTranscript, String> {
        let data = std::fs::read(Self::dir(cwd)?.join(Self::file_name(id)?))
            .map_err(|e| format!("No se pudo leer la conversación: {e}"))?;
        serde_json::from_slice(&data).map_err(|e| format!("Conversación inválida: {e}"))
    }

    fn delete(&self, cwd: &str, id: &str) -> Result<(), String> {
        match std::fs::remove_file(Self::dir(cwd)?.join(Self::file_name(id)?)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }

    fn delete_all(&self, cwd: &str) -> Result<(), String> {
        for path in Self::files(cwd)? {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn location(&self, cwd: &str, id: &str) -> Result<(PathBuf, String), String> {
        let dir = Self::dir(cwd)?;
        let name = Self::file_name(id)?;
        if !dir.join(&name).exists() {
            return Err("La conversación aún no se ha guardado".into());
        }
        Ok((dir, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn transcript(id: &str, cwd: &str, updated_at: i64) -> AgentTranscript {
        AgentTranscript {
            id: id.into(),
            agent_id: "claude-acp".into(),
            agent_name: "Claude".into(),
            cwd: cwd.into(),
            title: format!("t {id}"),
            acp_session_id: Some("s".into()),
            created_at: 1,
            updated_at,
            items: json!([{"type": "user", "text": "hola"}]),
        }
    }

    #[test]
    fn guarda_en_la_boveda_lista_carga_y_borra() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let (a, b) = (a.path().to_str().unwrap(), b.path().to_str().unwrap());
        let repo = AgentTranscriptRepositoryImpl;
        assert!(repo.list(a).unwrap().is_empty());
        repo.save(transcript("uno", a, 10)).unwrap();
        repo.save(transcript("dos", a, 20)).unwrap();
        repo.save(transcript("otra", b, 30)).unwrap();
        assert!(
            Path::new(a)
                .join(".synapse/agent-sessions/uno.json")
                .exists()
        );
        let ids: Vec<_> = repo.list(a).unwrap().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, ["dos", "uno"]);
        assert_eq!(repo.load(a, "uno").unwrap(), transcript("uno", a, 10));
        assert_eq!(repo.location(a, "uno").unwrap().1, "uno.json");
        repo.delete(a, "uno").unwrap();
        assert_eq!(repo.list(a).unwrap().len(), 1);
        assert!(repo.load(a, "../x").is_err());
        repo.delete_all(a).unwrap();
        assert!(repo.list(a).unwrap().is_empty());
        assert_eq!(repo.list(b).unwrap().len(), 1);
    }
}
