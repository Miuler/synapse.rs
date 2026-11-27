//! Conexiones a gestores de issues en `issues.connections` de `config.jsonc`. Leer no escribe nada.

use crate::domain::issue::connection::{self, Connection};
use crate::domain::issue::connection_repository::ConnectionRepository;
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind};
use crate::infrastructure::service::os::git_binary;
use serde_json::{Map, Value, json};
use std::path::PathBuf;

pub struct ConnectionRepositoryImpl {
    config_dir: PathBuf,
}

impl ConnectionRepositoryImpl {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }
}

/// Una entrada escrita a mano con un id o un servidor que no valen se ignora.
fn parse(entry: &Value) -> Option<Connection> {
    let text = |key: &str| {
        entry[key]
            .as_str()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let id = text("id").filter(|id| connection::is_valid_id(id))?;
    Some(Connection {
        provider: text("provider")?,
        name: text("name").unwrap_or_else(|| id.clone()),
        server: connection::normalize_server(&text("server")?).ok()?,
        user: text("user"),
        secret: text("secret"),
        id,
    })
}

impl ConnectionRepository for ConnectionRepositoryImpl {
    fn list(&self) -> Result<Vec<Connection>, IssueError> {
        let config = git_binary::read_config(&self.config_dir);
        let entries = config
            .pointer("/issues/connections")
            .and_then(Value::as_array);
        Ok(entries
            .map(|entries| entries.iter().filter_map(parse).collect())
            .unwrap_or_default())
    }

    fn save(&self, connections: &[Connection]) -> Result<(), IssueError> {
        let failed = |e: String| IssueError::new(IssueErrorKind::Request, e);
        // Un archivo que no se entiende no se sustituye por uno casi vacío
        if let Ok(raw) = std::fs::read_to_string(git_binary::config_path(&self.config_dir))
            && !serde_json_lenient::from_str::<Value>(&raw).is_ok_and(|v| v.is_object())
        {
            return Err(failed(
                "config.jsonc no se puede leer: corrígelo antes de guardar".into(),
            ));
        }
        let mut config = git_binary::read_config(&self.config_dir);
        let root = config
            .as_object_mut()
            .expect("read_config devuelve un objeto");
        let old = root
            .get("issues")
            .and_then(|i| i["connections"].as_array())
            .cloned()
            .unwrap_or_default();
        let entries: Vec<Value> = connections
            .iter()
            .map(|c| {
                // Lo que la entrada tuviera y no se interpreta se conserva
                let mut entry = old
                    .iter()
                    .find(|e| e["id"] == c.id.as_str())
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_else(Map::new);
                for (key, value) in [
                    ("id", Some(&c.id)),
                    ("provider", Some(&c.provider)),
                    ("name", Some(&c.name)),
                    ("server", Some(&c.server)),
                    ("user", c.user.as_ref()),
                    ("secret", c.secret.as_ref()),
                ] {
                    match value {
                        Some(v) => entry.insert(key.into(), json!(v)),
                        None => entry.remove(key),
                    };
                }
                Value::Object(entry)
            })
            .collect();
        if !root.get("issues").is_some_and(Value::is_object) {
            root.insert("issues".into(), json!({}));
        }
        let section = root["issues"].as_object_mut().expect("recién asegurado");
        if entries.is_empty() {
            section.remove("connections");
        } else {
            section.insert("connections".into(), Value::Array(entries));
        }
        if section.is_empty() {
            root.remove("issues");
        }
        git_binary::write_config(&self.config_dir, &config).map_err(failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leer_no_toca_el_archivo_y_guardar_conserva_lo_ajeno() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.jsonc");
        let original = r#"// mi comentario
{ "azure_devops": { "token": "op://b/e/c" },
  "issues": { "connections": [
    { "id": "a", "provider": "azure_devops", "server": "https://a.test/org/", "nota": "mía" },
    { "id": "../x", "provider": "azure_devops", "server": "https://a.test" },
    { "id": "b", "provider": "azure_devops", "server": "http://ajeno.test" }
  ] } }"#;
        std::fs::write(&path, original).unwrap();
        let repository = ConnectionRepositoryImpl::new(dir.path().into());
        let found = repository.list().unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(
            (found[0].name.as_str(), found[0].server.as_str()),
            ("a", "https://a.test/org")
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);

        repository.save(&found).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("op://b/e/c") && saved.contains("\"nota\": \"mía\""));

        repository.save(&[]).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(!saved.contains("issues") && saved.contains("azure_devops"));
    }

    #[test]
    fn un_token_pegado_no_llega_a_config_jsonc() {
        use crate::application::dto::issue::save_connection_command::SaveConnectionCommand;
        use crate::application::use_cases::issue::save_connection_use_case::SaveConnectionUseCase;
        use crate::infrastructure::service::http::azure_devops_client::AzureGlobalCredential;
        use crate::infrastructure::service::os::secret_store::SecretStore;

        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::File(dir.path().join("llavero"));
        let command = SaveConnectionCommand {
            provider: "azure_devops".into(),
            name: "Trabajo".into(),
            server: "https://dev.azure.com/org".into(),
            credential: Some("token-secreto".into()),
            ..Default::default()
        };
        let global = AzureGlobalCredential {
            config_dir: dir.path(),
            store: &store,
        };
        let view = SaveConnectionUseCase::new(
            ConnectionRepositoryImpl::new(dir.path().into()),
            store.clone(),
            global,
        )
        .execute(command, &["azure_devops".to_string()])
        .unwrap();
        let config = std::fs::read_to_string(dir.path().join("config.jsonc")).unwrap();
        assert!(config.contains("\"trabajo\"") && !config.contains("token-secreto"));
        assert!(
            !serde_json::to_string(&view)
                .unwrap()
                .contains("token-secreto")
        );
        assert_eq!(
            store.get("issues-trabajo").unwrap().as_deref(),
            Some("token-secreto")
        );
    }

    #[test]
    fn un_archivo_mal_escrito_no_se_sobrescribe() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.jsonc");
        std::fs::write(&path, "{ roto").unwrap();
        let repository = ConnectionRepositoryImpl::new(dir.path().into());
        assert!(repository.list().unwrap().is_empty());
        assert!(repository.save(&[]).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ roto");
    }
}
