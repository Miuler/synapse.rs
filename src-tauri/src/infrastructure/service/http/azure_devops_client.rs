//! API REST de Azure DevOps. El token se lee del entorno y solo viaja a los servidores de Azure
//! DevOps, o al que se indique en `AZURE_DEVOPS_URL`: la URL del proyecto sale de un archivo de la
//! bóveda, que puede venir de cualquiera.

use crate::application::service::work_item_tracker::{TrackerFuture, WorkItemTracker};
use crate::domain::work_item::work_item::WorkItem;
use crate::domain::work_item::work_item_error::{WorkItemError, WorkItemErrorKind as Kind};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use reqwest::{Method, Url};
use serde_json::{Value, json};
use std::time::Duration;

const API: &str = "api-version=7.1";
const COMMENTS_API: &str = "api-version=7.1-preview.3";
/// Máximo de ids por llamada a `workitemsbatch`.
const BATCH: usize = 200;

pub struct AzureDevOpsClient {
    token: Option<String>,
    /// Servidor propio (Azure DevOps Server) admitido además de los de Microsoft.
    server: Option<String>,
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
    pub fn from_env() -> Self {
        Self {
            token: env("AZURE_DEVOPS_PAT").or_else(|| env("ADO_PAT")),
            server: env("AZURE_DEVOPS_URL"),
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
        let token = self.token.as_deref().ok_or_else(|| {
            WorkItemError::new(Kind::MissingToken, "AZURE_DEVOPS_PAT, ADO_PAT")
        })?;
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
            let mut value = HeaderValue::from_str(token)
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
        interpret(status.as_u16(), &text)
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
            token: Some("falso".into()),
            server: server.map(str::to_string),
        }
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
            token: None,
            server: None,
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
