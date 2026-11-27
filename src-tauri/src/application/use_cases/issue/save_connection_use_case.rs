use crate::application::dto::issue::connection_view::ConnectionView;
use crate::application::dto::issue::save_connection_command::SaveConnectionCommand;
use crate::application::service::credential_store::{CredentialStore, GlobalCredential};
use crate::domain::issue::connection::{self, Connection};
use crate::domain::issue::connection_repository::ConnectionRepository;
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind as Kind};

/// Qué hacer con la credencial de la conexión que se guarda.
enum Credential {
    Keep,
    Remove,
    Reference(String),
    Token(String),
}

impl Credential {
    fn parse(value: &str) -> Result<Self, IssueError> {
        // «Copiar referencia secreta» de 1Password la entrega entre comillas
        let value = value.trim().trim_matches('"').trim();
        if value.is_empty() {
            Ok(Self::Remove)
        } else if value.starts_with("op://") {
            Ok(Self::Reference(value.to_string()))
        } else if value.starts_with("op:") {
            // Una referencia mal escrita no se toma por un token
            Err(IssueError::new(Kind::TokenReference, "op://"))
        } else {
            Ok(Self::Token(value.to_string()))
        }
    }
}

pub struct SaveConnectionUseCase<R, C, G>
where
    R: ConnectionRepository,
    C: CredentialStore,
    G: GlobalCredential,
{
    connections: R,
    credentials: C,
    global: G,
}

impl<R, C, G> SaveConnectionUseCase<R, C, G>
where
    R: ConnectionRepository,
    C: CredentialStore,
    G: GlobalCredential,
{
    pub fn new(connections: R, credentials: C, global: G) -> Self {
        Self {
            connections,
            credentials,
            global,
        }
    }

    /// Cambiar el servidor o el proveedor descarta la credencial: nunca se envía a otro sitio.
    pub fn execute(
        &self,
        command: SaveConnectionCommand,
        providers: &[String],
    ) -> Result<ConnectionView, IssueError> {
        let name = command.name.trim().to_string();
        if name.is_empty() {
            return Err(IssueError::new(Kind::Rule, "name"));
        }
        if !providers.contains(&command.provider) {
            return Err(IssueError::new(Kind::UnknownProvider, command.provider));
        }
        let server = connection::normalize_server(&command.server)?;
        let mut all = self.connections.list()?;
        let index = match &command.id {
            Some(id) => Some(
                all.iter()
                    .position(|c| &c.id == id)
                    .ok_or_else(|| IssueError::new(Kind::NoConnection, id))?,
            ),
            None => None,
        };
        let previous = index.map(|i| all[i].clone());
        let moved = previous
            .as_ref()
            .is_some_and(|p| p.server != server || p.provider != command.provider);

        let credential = match command.credential.as_deref() {
            Some(value) => Credential::parse(value)?,
            None if command.use_global => {
                match self.global.for_server(&command.provider, &server)? {
                    Some(value) => Credential::parse(&value)?,
                    None => return Err(IssueError::new(Kind::MissingToken, "global")),
                }
            }
            None => Credential::Keep,
        };

        let id = previous.as_ref().map(|p| p.id.clone()).unwrap_or_else(|| {
            let taken: Vec<&str> = all.iter().map(|c| c.id.as_str()).collect();
            connection::new_id(&name, &taken)
        });
        let mut saved = Connection {
            id,
            provider: command.provider,
            name,
            server,
            user: command.user.filter(|u| !u.trim().is_empty()),
            secret: previous.and_then(|p| p.secret).filter(|_| !moved),
        };
        let key = saved.keyring_name();
        let keyring = |detail| IssueError::new(Kind::Keyring, detail);
        match credential {
            Credential::Token(token) => {
                self.credentials.set(&key, &token).map_err(keyring)?;
                saved.secret = None;
            }
            Credential::Reference(reference) => {
                // Que el llavero no responda no impide usar 1Password
                let _ = self.credentials.delete(&key);
                saved.secret = Some(reference);
            }
            Credential::Remove => {
                self.credentials.delete(&key).map_err(keyring)?;
                saved.secret = None;
            }
            // Un token que quedara de una conexión borrada tampoco vale para una nueva
            Credential::Keep if moved || index.is_none() => {
                if saved.secret.is_none() {
                    self.credentials.delete(&key).map_err(keyring)?;
                }
            }
            Credential::Keep => {}
        }
        match index {
            Some(i) => all[i] = saved.clone(),
            None => all.push(saved.clone()),
        }
        self.connections.save(&all)?;
        let stored = saved.secret.is_none() && matches!(self.credentials.get(&key), Ok(Some(_)));
        Ok(ConnectionView::new(saved, stored))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::use_cases::issue::delete_connection_use_case::DeleteConnectionUseCase;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Memory {
        connections: Arc<Mutex<Vec<Connection>>>,
        keyring: Arc<Mutex<HashMap<String, String>>>,
    }

    impl ConnectionRepository for Memory {
        fn list(&self) -> Result<Vec<Connection>, IssueError> {
            Ok(self.connections.lock().unwrap().clone())
        }
        fn save(&self, connections: &[Connection]) -> Result<(), IssueError> {
            *self.connections.lock().unwrap() = connections.to_vec();
            Ok(())
        }
    }

    impl CredentialStore for Memory {
        fn get(&self, name: &str) -> Result<Option<String>, String> {
            Ok(self.keyring.lock().unwrap().get(name).cloned())
        }
        fn set(&self, name: &str, value: &str) -> Result<(), String> {
            self.keyring
                .lock()
                .unwrap()
                .insert(name.into(), value.into());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<(), String> {
            self.keyring.lock().unwrap().remove(name);
            Ok(())
        }
    }

    /// La global solo vale para `https://nube.test`.
    impl GlobalCredential for Memory {
        fn for_server(&self, _: &str, server: &str) -> Result<Option<String>, IssueError> {
            Ok(server
                .starts_with("https://nube.test")
                .then(|| "token-global".to_string()))
        }
    }

    fn command(name: &str, server: &str, credential: Option<&str>) -> SaveConnectionCommand {
        SaveConnectionCommand {
            provider: "azure_devops".into(),
            name: name.into(),
            server: server.into(),
            credential: credential.map(str::to_string),
            ..Default::default()
        }
    }

    fn save(memory: &Memory, command: SaveConnectionCommand) -> Result<ConnectionView, IssueError> {
        SaveConnectionUseCase::new(memory.clone(), memory.clone(), memory.clone())
            .execute(command, &["azure_devops".to_string()])
    }

    fn token(memory: &Memory, id: &str) -> Option<String> {
        memory.get(&format!("issues-{id}")).unwrap()
    }

    #[test]
    fn dos_conexiones_del_mismo_proveedor_son_independientes() {
        let memory = Memory::default();
        let work = save(
            &memory,
            command("Trabajo", "https://a.test/org", Some("uno")),
        )
        .unwrap();
        let client = save(
            &memory,
            command("Cliente", "https://b.test/org", Some("dos")),
        )
        .unwrap();
        assert_eq!(
            (work.id.as_str(), client.id.as_str()),
            ("trabajo", "cliente")
        );
        assert!(work.stored && client.stored);
        assert_eq!(token(&memory, "trabajo").as_deref(), Some("uno"));
        assert_eq!(token(&memory, "cliente").as_deref(), Some("dos"));
        // Ni la respuesta ni lo guardado en la configuración llevan el token
        let json = serde_json::to_string(&[work, client]).unwrap();
        assert!(!json.contains("uno") && !json.contains("dos"));
        assert!(memory.list().unwrap().iter().all(|c| c.secret.is_none()));

        DeleteConnectionUseCase::new(memory.clone(), memory.clone())
            .execute("trabajo")
            .unwrap();
        assert_eq!(token(&memory, "trabajo"), None);
        assert_eq!(token(&memory, "cliente").as_deref(), Some("dos"));
        assert_eq!(memory.list().unwrap().len(), 1);
    }

    #[test]
    fn cambiar_el_servidor_descarta_la_credencial() {
        let memory = Memory::default();
        let saved = save(&memory, command("A", "https://a.test/org", Some("uno"))).unwrap();
        let edit = |server: &str, credential: Option<&str>| {
            let mut edit = command("A renombrada", server, credential);
            edit.id = Some(saved.id.clone());
            save(&memory, edit).unwrap()
        };
        // El mismo servidor escrito de otra forma no es un cambio
        let same = edit("https://a.test/org/", None);
        assert!(same.stored && same.id == "a");
        let moved = edit("https://otro.test/org", None);
        assert!(!moved.stored);
        assert_eq!(token(&memory, "a"), None);

        let referenced = edit("https://otro.test/org", Some("\"op://b/e/c\""));
        assert_eq!(referenced.reference.as_deref(), Some("op://b/e/c"));
        let moved = edit("https://a.test/org", None);
        assert_eq!((moved.reference, moved.stored), (None, false));
    }

    #[test]
    fn la_credencial_global_solo_se_copia_a_su_servidor() {
        let memory = Memory::default();
        let mut elsewhere = command("Ajena", "https://ajeno.test/org", None);
        elsewhere.use_global = true;
        assert_eq!(
            save(&memory, elsewhere).unwrap_err().kind,
            Kind::MissingToken
        );
        assert!(memory.list().unwrap().is_empty());

        let mut cloud = command("Nube", "https://nube.test/org", None);
        cloud.use_global = true;
        assert!(save(&memory, cloud).unwrap().stored);
        assert_eq!(token(&memory, "nube").as_deref(), Some("token-global"));
    }

    #[test]
    fn lo_que_no_vale_no_guarda_nada() {
        let memory = Memory::default();
        for (command, kind) in [
            (command(" ", "https://a.test", None), Kind::Rule),
            (command("A", "http://a.test", None), Kind::InvalidUrl),
            (
                command("A", "https://a.test", Some("op:/mal")),
                Kind::TokenReference,
            ),
        ] {
            assert_eq!(save(&memory, command).unwrap_err().kind, kind);
        }
        assert!(memory.list().unwrap().is_empty());
        assert!(memory.keyring.lock().unwrap().is_empty());
    }
}
