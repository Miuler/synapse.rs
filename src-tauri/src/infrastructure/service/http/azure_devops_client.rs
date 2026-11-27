//! API REST de Azure DevOps. El token sale de 1Password (`azure_devops.token` de `config.jsonc`,
//! una referencia `op://`), del llavero del sistema o del entorno, y solo viaja a los servidores de
//! Azure DevOps, o al que se indique en `AZURE_DEVOPS_URL`: la URL del proyecto sale de un archivo
//! de la bóveda, que puede venir de cualquiera.

use crate::application::service::work_item_tracker::{TrackerFuture, WorkItemTracker};
use crate::domain::work_item::work_item::WorkItem;
use crate::domain::work_item::work_item_error::{WorkItemError, WorkItemErrorKind as Kind};
use crate::infrastructure::service::os::secret_store::SecretStore;
use crate::infrastructure::service::os::{git_binary, one_password};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use reqwest::{Method, Url};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

const API: &str = "api-version=7.1";
const COMMENTS_API: &str = "api-version=7.1-preview.3";
/// Máximo de ids por llamada a `workitemsbatch`.
const BATCH: usize = 200;
/// Nombre del token en el llavero.
const KEYRING_NAME: &str = "azure-devops-token";

/// De dónde sale el token.
#[derive(Debug, PartialEq)]
enum Token {
    /// Referencia `op://` que se lee en 1Password justo antes de enviar.
    Reference(String),
    /// Lo configurado no es una referencia. No se guarda: puede ser un token pegado tal cual.
    NotAReference,
    /// Sin referencia configurada: el del llavero o, si no hay, el del entorno.
    Local { env: Option<String> },
}

impl Token {
    /// La configuración de Synapse manda sobre el entorno.
    fn choose(configured: Option<String>, env: Option<String>) -> Self {
        match configured {
            Some(c) if c.starts_with("op://") => Self::Reference(c),
            Some(_) => Self::NotAReference,
            None => Self::Local { env },
        }
    }

    /// La referencia de 1Password de la que saldría el token.
    fn reference(&self) -> Option<&str> {
        match self {
            Self::Reference(r) => Some(r),
            Self::Local { env: Some(e) } if e.starts_with("op://") => Some(e),
            _ => None,
        }
    }
}

/// `azure_devops.token` de `config.jsonc`, tal como está.
fn configured_token(config_dir: &Path) -> Option<String> {
    git_binary::read_config(config_dir)
        .pointer("/azure_devops/token")
        .and_then(Value::as_str)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Lo que la interfaz puede saber del token: nunca su valor.
#[derive(Debug, PartialEq, Serialize)]
pub struct TokenSetting {
    /// Referencia de 1Password configurada.
    pub reference: Option<String>,
    /// Hay un token guardado en el llavero.
    pub stored: bool,
}

fn keyring_error(detail: String) -> WorkItemError {
    WorkItemError::new(Kind::Keyring, detail)
}

pub fn token_setting(config_dir: &Path, store: &SecretStore) -> Result<TokenSetting, WorkItemError> {
    // Algo escrito a mano que no sea una referencia (un token) no sale de aquí
    let reference = configured_token(config_dir).filter(|v| v.starts_with("op://"));
    // Con referencia, el llavero no se consulta: podría pedir que se desbloquee
    let stored = reference.is_none()
        && store
            .get(KEYRING_NAME)
            .map_err(keyring_error)?
            .is_some();
    Ok(TokenSetting { reference, stored })
}

/// Una referencia `op://` va a `config.jsonc`; un token, al llavero; vacío, se quitan los dos.
pub fn save_token(
    config_dir: &Path,
    store: &SecretStore,
    value: &str,
) -> Result<(), WorkItemError> {
    // «Copiar referencia secreta» de 1Password la entrega entre comillas
    let value = value.trim().trim_matches('"').trim();
    if value.starts_with("op://") {
        save_reference(config_dir, Some(value))?;
        // Que el llavero no responda no impide usar 1Password
        let _ = store.delete(KEYRING_NAME);
        return Ok(());
    }
    // Una referencia mal escrita no se toma por un token
    if value.starts_with("op:") {
        return Err(WorkItemError::new(
            Kind::TokenReference,
            "azure_devops.token",
        ));
    }
    if value.is_empty() {
        store.delete(KEYRING_NAME).map_err(keyring_error)?;
    } else {
        store.set(KEYRING_NAME, value).map_err(keyring_error)?;
    }
    save_reference(config_dir, None)
}

/// Pone o quita `azure_devops.token` sin tocar lo demás.
fn save_reference(config_dir: &Path, reference: Option<&str>) -> Result<(), WorkItemError> {
    if reference.is_none() && configured_token(config_dir).is_none() {
        return Ok(());
    }
    let mut config = git_binary::read_config(config_dir);
    let root = config
        .as_object_mut()
        .expect("read_config devuelve un objeto");
    if !root.get("azure_devops").is_some_and(Value::is_object) {
        root.insert("azure_devops".into(), json!({}));
    }
    let section = root["azure_devops"]
        .as_object_mut()
        .expect("recién asegurado");
    match reference {
        Some(r) => section.insert("token".into(), json!(r)),
        None => section.remove("token"),
    };
    if section.is_empty() {
        root.remove("azure_devops");
    }
    git_binary::write_config(config_dir, &config).map_err(|e| WorkItemError::new(Kind::Request, e))
}

pub struct AzureDevOpsClient {
    token: Token,
    /// Servidor propio (Azure DevOps Server) admitido además de los de Microsoft.
    server: Option<String>,
    /// Ejecutable del CLI de 1Password.
    op: String,
    store: SecretStore,
}

/// Colección u organización (`https://dev.azure.com/org`) y el proyecto dentro de ella.
#[derive(Debug, PartialEq)]
struct Project {
    collection: String,
    name: String,
}

impl Project {
    fn url(&self, path: &str) -> String {
        format!("{}/{}/_apis/wit/{path}", self.collection, self.name)
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl AzureDevOpsClient {
    /// Se lee en cada acción: cambiar la configuración no pide reiniciar.
    pub fn configured(config_dir: &Path) -> Self {
        Self {
            token: Token::choose(
                configured_token(config_dir),
                env("AZURE_DEVOPS_PAT").or_else(|| env("ADO_PAT")),
            ),
            server: env("AZURE_DEVOPS_URL"),
            op: env("SYNAPSE_OP_PATH").unwrap_or_else(|| "op".to_string()),
            store: SecretStore::from_env(),
        }
    }

    async fn token(&self) -> Result<String, WorkItemError> {
        let env = match &self.token {
            Token::NotAReference => {
                return Err(WorkItemError::new(
                    Kind::TokenReference,
                    "azure_devops.token",
                ));
            }
            Token::Reference(_) => None,
            Token::Local { env } => {
                let store = self.store.clone();
                let stored = tokio::task::spawn_blocking(move || store.get(KEYRING_NAME))
                    .await
                    .map_err(|e| keyring_error(e.to_string()))?;
                match (stored, env) {
                    (Ok(Some(token)), _) => return Ok(token),
                    // Sin llavero, la variable de entorno sigue valiendo
                    (_, Some(env)) => Some(env.clone()),
                    (Err(e), None) => return Err(keyring_error(e)),
                    (Ok(None), None) => {
                        return Err(WorkItemError::new(
                            Kind::MissingToken,
                            "AZURE_DEVOPS_PAT, ADO_PAT",
                        ));
                    }
                }
            }
        };
        match (self.token.reference(), env) {
            (Some(reference), _) => one_password::read(&self.op, reference)
                .await
                .map_err(|e| WorkItemError::new(Kind::OnePassword, e)),
            (None, env) => Ok(env.unwrap_or_default()),
        }
    }

    fn project(&self, project_url: &str) -> Result<Project, WorkItemError> {
        let invalid = || WorkItemError::new(Kind::InvalidUrl, project_url);
        let url = Url::parse(project_url.trim()).map_err(|_| invalid())?;
        if !url.username().is_empty() || url.password().is_some() {
            return Err(invalid());
        }
        let host = url.host_str().ok_or_else(invalid)?;
        let segments: Vec<&str> = url
            .path_segments()
            .ok_or_else(invalid)?
            .filter(|s| !s.is_empty())
            .collect();
        let origin = url.origin().ascii_serialization();
        let microsoft = url.scheme() == "https" && url.port().is_none();
        let own_server = self
            .server
            .as_deref()
            .and_then(|s| Url::parse(s).ok())
            .is_some_and(|s| s.origin() == url.origin());

        // Cuántos tramos forman la colección y cuál es el proyecto
        let (collection, name) = if microsoft && host == "dev.azure.com" {
            (1, 1)
        } else if microsoft && host.ends_with(".visualstudio.com") {
            (0, 0)
        } else if own_server {
            let last = segments.len().checked_sub(1).ok_or_else(invalid)?;
            (last, last)
        } else {
            return Err(WorkItemError::new(Kind::HostNotAllowed, origin));
        };
        let name = segments.get(name).ok_or_else(invalid)?.to_string();
        let mut base = origin;
        for segment in &segments[..collection] {
            base.push('/');
            base.push_str(segment);
        }
        Ok(Project {
            collection: base,
            name,
        })
    }

    async fn send(
        &self,
        method: Method,
        url: &str,
        body: Option<(&str, Value)>,
    ) -> Result<Value, WorkItemError> {
        let token = self.token().await?;
        let failed = |e: reqwest::Error| WorkItemError::new(Kind::Request, e.to_string());
        // Sin redirecciones: Azure DevOps redirige al inicio de sesión cuando el token no vale
        let client = reqwest::Client::builder()
            .user_agent("Synapse")
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(failed)?;
        let mut request = client.request(method, url);
        request = if token.starts_with("Basic ") {
            let mut value = HeaderValue::from_str(&token)
                .map_err(|_| WorkItemError::new(Kind::Auth, ""))?;
            value.set_sensitive(true);
            request.header(AUTHORIZATION, value)
        } else {
            request.basic_auth("", Some(token))
        };
        if let Some((content_type, body)) = body {
            request = request
                .header(CONTENT_TYPE, content_type)
                .body(body.to_string());
        }
        let response = request.send().await.map_err(failed)?;
        let status = response.status();
        let text = response.text().await.map_err(failed)?;
        let result = interpret(status.as_u16(), &text);
        // Un token rechazado puede haberse cambiado en 1Password: la próxima vez se vuelve a leer
        if let (Some(reference), Err(error)) = (self.token.reference(), &result)
            && error.kind == Kind::Auth
        {
            one_password::forget(&self.op, reference).await;
        }
        result
    }
}

fn interpret(status: u16, text: &str) -> Result<Value, WorkItemError> {
    // 203: Azure DevOps responde así con la página de inicio de sesión
    if status == 401 || status == 203 || (300..400).contains(&status) {
        return Err(WorkItemError::new(Kind::Auth, format!("HTTP {status}")));
    }
    let body: Option<Value> = serde_json::from_str(text).ok();
    let field = |name: &str| {
        body.as_ref()
            .and_then(|b| b[name].as_str())
            .unwrap_or_default()
    };
    let (message, type_key) = (field("message"), field("typeKey"));
    let success = (200..300).contains(&status);
    let kind = if status == 404
        || message.contains("TF401232")
        || type_key == "WorkItemUnauthorizedAccessException"
    {
        Kind::NotFound
    } else if status == 409 || status == 412 || message.contains("TF26071") {
        Kind::Conflict
    } else if type_key == "RuleValidationException" {
        Kind::Rule
    } else if !success {
        Kind::Request
    } else {
        return body.ok_or_else(|| WorkItemError::new(Kind::Auth, "HTML"));
    };
    let detail = if message.is_empty() {
        format!("HTTP {status}")
    } else {
        message.to_string()
    };
    Err(WorkItemError::new(kind, detail))
}

fn work_item(value: &Value) -> Option<WorkItem> {
    let field = |name: &str| value["fields"][name].as_str().unwrap_or_default().to_string();
    Some(WorkItem {
        id: value["id"].as_u64()?,
        rev: value["rev"].as_u64()?,
        title: field("System.Title"),
        state: field("System.State"),
        work_item_type: field("System.WorkItemType"),
    })
}

fn expect_work_item(value: &Value) -> Result<WorkItem, WorkItemError> {
    work_item(value).ok_or_else(|| {
        WorkItemError::new(Kind::Request, "respuesta sin work item")
    })
}

fn set_field(field: &str, value: &str) -> Value {
    json!({"op": "add", "path": format!("/fields/{field}"), "value": value})
}

/// Los comentarios de Azure DevOps son HTML.
fn comment_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', "<br>")
}

impl WorkItemTracker for AzureDevOpsClient {
    fn query<'a>(&'a self, project_url: &'a str, query: &'a str) -> TrackerFuture<'a, Vec<u64>> {
        Box::pin(async move {
            let url = self.project(project_url)?.url(&format!("wiql?{API}"));
            let body = ("application/json", json!({ "query": query }));
            let response = self.send(Method::POST, &url, Some(body)).await?;
            let items = response["workItems"].as_array();
            Ok(items
                .map(|items| items.iter().filter_map(|i| i["id"].as_u64()).collect())
                .unwrap_or_default())
        })
    }

    fn work_items<'a>(
        &'a self,
        project_url: &'a str,
        ids: &'a [u64],
    ) -> TrackerFuture<'a, Vec<WorkItem>> {
        Box::pin(async move {
            let project = self.project(project_url)?;
            let url = format!("{}/_apis/wit/workitemsbatch?{API}", project.collection);
            let mut found = Vec::new();
            for chunk in ids.chunks(BATCH) {
                let body = json!({
                    "ids": chunk,
                    "fields": ["System.Title", "System.State", "System.WorkItemType"],
                    "errorPolicy": "Omit",
                });
                let response = self
                    .send(Method::POST, &url, Some(("application/json", body)))
                    .await?;
                // Con `Omit`, los que no se pueden leer faltan o llegan como `null`
                if let Some(items) = response["value"].as_array() {
                    found.extend(items.iter().filter_map(work_item));
                }
            }
            Ok(found)
        })
    }

    fn update<'a>(
        &'a self,
        project_url: &'a str,
        id: u64,
        rev: u64,
        title: Option<&'a str>,
        state: Option<&'a str>,
    ) -> TrackerFuture<'a, WorkItem> {
        Box::pin(async move {
            let url = self
                .project(project_url)?
                .url(&format!("workitems/{id}?{API}"));
            let mut patch = vec![json!({"op": "test", "path": "/rev", "value": rev})];
            patch.extend(title.map(|t| set_field("System.Title", t)));
            patch.extend(state.map(|s| set_field("System.State", s)));
            let body = ("application/json-patch+json", Value::Array(patch));
            expect_work_item(&self.send(Method::PATCH, &url, Some(body)).await?)
        })
    }

    fn create<'a>(
        &'a self,
        project_url: &'a str,
        work_item_type: &'a str,
        title: &'a str,
        assigned_to: Option<&'a str>,
    ) -> TrackerFuture<'a, WorkItem> {
        Box::pin(async move {
            let base = self.project(project_url)?.url("workitems");
            let mut url =
                Url::parse(&base).map_err(|_| WorkItemError::new(Kind::InvalidUrl, project_url))?;
            if let Ok(mut path) = url.path_segments_mut() {
                path.push(&format!("${work_item_type}"));
            }
            url.set_query(Some(API));
            let mut patch = vec![set_field("System.Title", title)];
            patch.extend(assigned_to.map(|a| set_field("System.AssignedTo", a)));
            let body = ("application/json-patch+json", Value::Array(patch));
            expect_work_item(&self.send(Method::POST, url.as_str(), Some(body)).await?)
        })
    }

    fn add_comment<'a>(
        &'a self,
        project_url: &'a str,
        id: u64,
        text: &'a str,
    ) -> TrackerFuture<'a, ()> {
        Box::pin(async move {
            let url = self
                .project(project_url)?
                .url(&format!("workItems/{id}/comments?{COMMENTS_API}"));
            let body = ("application/json", json!({ "text": comment_html(text) }));
            self.send(Method::POST, &url, Some(body)).await?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    fn client(server: Option<&str>) -> AzureDevOpsClient {
        AzureDevOpsClient {
            token: Token::Local {
                env: Some("falso".into()),
            },
            server: server.map(str::to_string),
            op: "op".into(),
            // Una carpeta que no existe: un llavero vacío
            store: SecretStore::File("/no/existe/llavero".into()),
        }
    }

    #[test]
    fn la_referencia_configurada_manda_y_un_token_en_la_configuracion_no_vale() {
        let some = |v: &str| Some(v.to_string());
        assert_eq!(
            Token::choose(some("op://b/e/c"), some("del-entorno")),
            Token::Reference("op://b/e/c".into())
        );
        assert_eq!(
            Token::choose(None, some("op://b/e/c")).reference(),
            Some("op://b/e/c")
        );
        assert_eq!(Token::choose(None, some("del-entorno")).reference(), None);
        assert_eq!(
            Token::choose(some("pegado-tal-cual"), None),
            Token::NotAReference
        );
    }

    #[test]
    fn lee_la_referencia_de_la_configuracion() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.jsonc"),
            "// comentario\n{ \"azure_devops\": { \"token\": \"op://b/e/c\" } }",
        )
        .unwrap();
        assert_eq!(
            AzureDevOpsClient::configured(dir.path()).token,
            Token::Reference("op://b/e/c".into())
        );
    }

    #[test]
    fn cada_valor_va_a_su_sitio_sin_tocar_el_resto() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.jsonc");
        let store = SecretStore::File(dir.path().join("llavero"));
        let read = || std::fs::read_to_string(&config).unwrap();
        let setting = || token_setting(dir.path(), &store).unwrap();
        std::fs::write(&config, r#"{ "git": { "path": "/usr/bin/git" } }"#).unwrap();

        // Un token pegado va al llavero y no a la configuración
        save_token(dir.path(), &store, " pegado-tal-cual\n").unwrap();
        assert_eq!(
            store.get(KEYRING_NAME).unwrap().as_deref(),
            Some("pegado-tal-cual")
        );
        assert!(!read().contains("pegado"));
        assert_eq!(
            setting(),
            TokenSetting {
                reference: None,
                stored: true
            }
        );

        // Una referencia lo sustituye
        save_token(dir.path(), &store, " \"op://b/Un elemento/c\"\n").unwrap();
        assert_eq!(
            setting(),
            TokenSetting {
                reference: Some("op://b/Un elemento/c".into()),
                stored: false
            }
        );
        assert_eq!(store.get(KEYRING_NAME).unwrap(), None);

        // Y un token sustituye a la referencia
        save_token(dir.path(), &store, "otro").unwrap();
        assert!(setting().stored && !read().contains("azure_devops"));

        let error = save_token(dir.path(), &store, "op:/mal").unwrap_err();
        assert_eq!(error.kind, Kind::TokenReference);

        save_token(dir.path(), &store, "").unwrap();
        assert_eq!(
            setting(),
            TokenSetting {
                reference: None,
                stored: false
            }
        );
        assert!(read().contains("/usr/bin/git"));

        // Escrito a mano, un token no se entrega a la interfaz
        std::fs::write(&config, r#"{ "azure_devops": { "token": "pegado" } }"#).unwrap();
        assert_eq!(setting().reference, None);
    }

    #[tokio::test]
    async fn el_token_del_llavero_manda_sobre_el_del_entorno() {
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::File(dir.path().to_path_buf());
        let with = |env: Option<&str>| AzureDevOpsClient {
            token: Token::Local {
                env: env.map(str::to_string),
            },
            server: None,
            op: "/no/existe/op".into(),
            store: store.clone(),
        };
        assert_eq!(with(Some("del-entorno")).token().await.unwrap(), "del-entorno");
        store.set(KEYRING_NAME, "del-llavero").unwrap();
        assert_eq!(with(Some("del-entorno")).token().await.unwrap(), "del-llavero");
        assert_eq!(with(None).token().await.unwrap(), "del-llavero");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pide_el_token_a_1password_una_vez_y_de_nuevo_si_se_rechaza() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let calls = dir.path().join("llamadas");
        let op = dir.path().join("op");
        std::fs::write(
            &op,
            format!("#!/bin/sh\necho \"$@\" >> '{}'\nprintf falso\n", calls.display()),
        )
        .unwrap();
        std::fs::set_permissions(&op, std::fs::Permissions::from_mode(0o755)).unwrap();
        let with_op = |server: &str| AzureDevOpsClient {
            token: Token::Reference("op://b/e/c".into()),
            server: Some(server.to_string()),
            op: op.to_string_lossy().into_owned(),
            store: SecretStore::File("/no/existe/llavero".into()),
        };
        let count = || std::fs::read_to_string(&calls).unwrap().lines().count();

        // Un servidor ajeno no llega a pedir el token
        let error = with_op("http://otro.local")
            .query("https://ejemplo.com/org/p", "SELECT")
            .await
            .unwrap_err();
        assert_eq!(error.kind, Kind::HostNotAllowed);
        assert!(!calls.exists());

        let ok = "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\n{}";
        let denied = "HTTP/1.1 401 Unauthorized\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
        for (reply, reads) in [(ok, 1), (ok, 1), (denied, 1), (ok, 2)] {
            let (origin, request) = serve(reply);
            let _ = with_op(&origin)
                .query(&format!("{origin}/org/p"), "SELECT")
                .await;
            assert!(request.join().unwrap().contains("authorization: Basic OmZhbHNv"));
            assert_eq!(count(), reads);
        }
        assert!(
            std::fs::read_to_string(&calls)
                .unwrap()
                .starts_with("read --no-newline op://b/e/c\n")
        );
    }

    #[tokio::test]
    async fn un_fallo_de_1password_se_explica() {
        let client = AzureDevOpsClient {
            token: Token::Reference("op://b/e/c".into()),
            server: None,
            op: "/no/existe/op".into(),
            store: SecretStore::File("/no/existe/llavero".into()),
        };
        let error = client
            .query("https://dev.azure.com/org/Proyecto", "SELECT")
            .await
            .unwrap_err();
        assert_eq!(error.kind, Kind::OnePassword);
        assert!(error.detail.contains("/no/existe/op"));
    }

    #[test]
    fn solo_admite_los_servidores_de_azure_devops_y_el_configurado() {
        let azure = client(None);
        let project = |url| azure.project(url);
        assert_eq!(
            project("https://dev.azure.com/org/Mi%20Proyecto/_boards").unwrap(),
            Project {
                collection: "https://dev.azure.com/org".into(),
                name: "Mi%20Proyecto".into()
            }
        );
        assert_eq!(
            project("https://org.visualstudio.com/Mi Proyecto").unwrap(),
            Project {
                collection: "https://org.visualstudio.com".into(),
                name: "Mi%20Proyecto".into()
            }
        );
        for url in [
            "https://ejemplo.com/org/proyecto",
            "http://dev.azure.com/org/proyecto",
            "https://dev.azure.com.ejemplo.com/org/proyecto",
            "https://dev.azure.com:8443/org/proyecto",
        ] {
            assert_eq!(project(url).unwrap_err().kind, Kind::HostNotAllowed, "{url}");
        }
        for url in [
            "https://dev.azure.com@ejemplo.com/org/proyecto",
            "https://dev.azure.com/org",
            "proyecto",
        ] {
            assert_eq!(project(url).unwrap_err().kind, Kind::InvalidUrl, "{url}");
        }

        let own = client(Some("http://tfs.local:8080/tfs"));
        assert_eq!(
            own.project("http://tfs.local:8080/tfs/Coleccion/Proyecto")
                .unwrap(),
            Project {
                collection: "http://tfs.local:8080/tfs/Coleccion".into(),
                name: "Proyecto".into()
            }
        );
        assert!(own.project("http://tfs.local:9090/Coleccion/Proyecto").is_err());
    }

    /// Atiende una petición con `reply` y devuelve lo que se recibió.
    fn serve(reply: &'static str) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut received = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                received.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&received);
                let complete = text.split_once("\r\n\r\n").is_some_and(|(head, body)| {
                    let length = head
                        .to_lowercase()
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:")?.trim().parse().ok())
                        .unwrap_or(0);
                    body.len() >= length
                });
                if n == 0 || complete {
                    break;
                }
            }
            stream.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&received).into_owned()
        });
        (origin, handle)
    }

    #[tokio::test]
    async fn actualiza_solo_sobre_la_revision_leida() {
        let (origin, request) = serve(
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 81\r\n\r\n{\"id\":5,\"rev\":3,\"fields\":{\"System.Title\":\"T\",\"System.State\":\"Done\",\"x\":\"123456\"}}",
        );
        let item = client(Some(&origin))
            .update(&format!("{origin}/org/Proyecto"), 5, 2, None, Some("Done"))
            .await
            .unwrap();
        assert_eq!((item.rev, item.state.as_str()), (3, "Done"));
        let request = request.join().unwrap();
        assert!(request.starts_with("PATCH /org/Proyecto/_apis/wit/workitems/5?api-version=7.1 "));
        assert!(request.contains("authorization: Basic OmZhbHNv"));
        assert!(request.contains("application/json-patch+json"));
        assert!(request.contains(r#"{"op":"test","path":"/rev","value":2}"#));
        assert!(!request.contains("System.Title\""));
    }

    #[tokio::test]
    async fn una_redireccion_es_un_error_de_autenticacion_y_no_se_sigue() {
        let (origin, request) = serve(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/login\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        );
        let error = client(Some(&origin))
            .query(&format!("{origin}/org/Proyecto"), "SELECT")
            .await
            .unwrap_err();
        assert_eq!(error.kind, Kind::Auth);
        request.join().unwrap();
    }

    #[tokio::test]
    async fn sin_token_no_se_envia_nada() {
        let client = AzureDevOpsClient {
            token: Token::Local { env: None },
            server: None,
            op: "op".into(),
            store: SecretStore::File("/no/existe/llavero".into()),
        };
        let error = client
            .query("https://dev.azure.com/org/Proyecto", "SELECT")
            .await
            .unwrap_err();
        assert_eq!(error.kind, Kind::MissingToken);
    }

    #[test]
    fn interpreta_las_respuestas_de_error() {
        let kind = |status, body| interpret(status, body).unwrap_err().kind;
        assert_eq!(kind(203, "<html>"), Kind::Auth);
        assert_eq!(kind(200, "<!DOCTYPE html>"), Kind::Auth);
        assert_eq!(kind(404, ""), Kind::NotFound);
        assert_eq!(kind(400, r#"{"message":"TF401232: no existe"}"#), Kind::NotFound);
        assert_eq!(kind(412, "{}"), Kind::Conflict);
        assert_eq!(kind(400, r#"{"typeKey":"RuleValidationException"}"#), Kind::Rule);
        assert_eq!(kind(500, ""), Kind::Request);
    }
}
