//! API REST de Azure DevOps. Cada conexión tiene su servidor y su token (en 1Password o en el
//! llavero), y el token solo viaja a ese servidor: la URL del proyecto sale de un archivo de la
//! bóveda, que puede venir de cualquiera. Sin conexiones configuradas queda la implícita: la nube de
//! Azure DevOps con la credencial única anterior (`azure_devops.token`, llavero o entorno).

use crate::application::service::credential_store::GlobalCredential;
use crate::application::service::issue_provider::{IssueProvider, ProviderFuture};
use crate::domain::issue::connection::{self, Connection};
use crate::domain::issue::filter::{CURRENT, Criterion, IssueFilter, ME};
use crate::domain::issue::issue::{
    Account, EntryKind, FieldChange, Issue, IssueChanges, IssueEntry, IssueState, NewIssue,
    Person, StateCategory, TypeStates,
};
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind as Kind};
use crate::domain::issue::provider_descriptor::{
    Capabilities, ConflictCheck, ConnectionField, ProviderDescriptor, StateChange,
};
use crate::infrastructure::service::http::{azure_devops_wiql, html_markdown};
use crate::infrastructure::service::os::secret_store::SecretStore;
use crate::infrastructure::service::os::{git_binary, one_password};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use reqwest::{Method, Url};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

/// Valor de `provider` de este adaptador.
pub const PROVIDER: &str = "azure_devops";
const API: &str = "api-version=7.1";
const DESCRIPTION: &str = "System.Description";
/// El texto principal de un error (Bug).
const REPRO_STEPS: &str = "Microsoft.VSTS.TCM.ReproSteps";
/// Los cambios del histórico que se guardan; el resto son contadores y fechas internas.
const HISTORY_FIELDS: [&str; 11] = [
    "System.WorkItemType",
    "System.Title",
    "System.State",
    "System.Reason",
    "System.AssignedTo",
    "System.IterationPath",
    "System.AreaPath",
    "System.Tags",
    "Microsoft.VSTS.Common.Priority",
    DESCRIPTION,
    REPRO_STEPS,
];
/// Entradas por página de comentarios y de cambios.
const PAGE: usize = 200;
/// Tope de páginas de una tarea: ningún servidor puede dejar pidiendo sin fin.
const MAX_PAGES: usize = 50;
/// Los campos de un work item que entran en el modelo común.
const FIELDS: [&str; 8] = [
    "System.Title",
    "System.State",
    "System.WorkItemType",
    "System.AssignedTo",
    "System.ChangedDate",
    DESCRIPTION,
    REPRO_STEPS,
    COMMENT_COUNT,
];
/// Cuántos comentarios tiene: delata uno nuevo aunque no cambie la revisión.
const COMMENT_COUNT: &str = "System.CommentCount";
/// Lo abierto en el proyecto y relacionado con quien sincroniza.
const DEFAULT_QUERY: &str = "SELECT [System.Id] FROM WorkItems
WHERE [System.TeamProject] = @project
  AND [System.State] NOT IN ('Closed', 'Cancelled', 'Removed', 'Done')
  AND ([System.AssignedTo] = @me OR [System.ChangedBy] = @me OR [System.CreatedBy] = @me)
ORDER BY [System.ChangedDate] DESC";
const COMMENTS_API: &str = "api-version=7.1-preview.3";
/// La lectura admite además el texto ya convertido a HTML.
const COMMENT_LIST_API: &str = "api-version=7.1-preview.4";
/// Máximo de ids por llamada a `workitemsbatch`.
const BATCH: usize = 200;
/// Nombre del token en el llavero.
const KEYRING_NAME: &str = "azure-devops-token";

/// De dónde sale el token.
#[derive(Debug, PartialEq)]
enum Token {
    /// Referencia `op://` que se lee en 1Password justo antes de enviar.
    Reference(String),
    /// Lo configurado (la clave que se indica) no es una referencia: puede ser un token pegado.
    NotAReference(&'static str),
    /// Sin referencia configurada: el del llavero o, si no hay, el del entorno.
    Local { env: Option<String> },
    /// El de una conexión, guardado en el llavero.
    Stored { keyring: String, connection: String },
}

impl Token {
    /// La configuración de Synapse manda sobre el entorno.
    fn choose(configured: Option<String>, env: Option<String>) -> Self {
        match configured {
            Some(c) if c.starts_with("op://") => Self::Reference(c),
            Some(_) => Self::NotAReference("azure_devops.token"),
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

fn keyring_error(detail: String) -> IssueError {
    IssueError::new(Kind::Keyring, detail)
}

pub fn token_setting(config_dir: &Path, store: &SecretStore) -> Result<TokenSetting, IssueError> {
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
) -> Result<(), IssueError> {
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
        return Err(IssueError::new(
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
fn save_reference(config_dir: &Path, reference: Option<&str>) -> Result<(), IssueError> {
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
    git_binary::write_config(config_dir, &config).map_err(|e| IssueError::new(Kind::Request, e))
}

/// Los servidores de Microsoft, a los que vale la credencial única anterior.
fn is_cloud(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url
            .host_str()
            .is_some_and(|h| h == "dev.azure.com" || h.ends_with(".visualstudio.com"))
}

/// La credencial única anterior a las conexiones.
pub struct AzureGlobalCredential<'a> {
    pub config_dir: &'a Path,
    pub store: &'a SecretStore,
}

impl GlobalCredential for AzureGlobalCredential<'_> {
    fn for_server(&self, provider: &str, server: &str) -> Result<Option<String>, IssueError> {
        if provider != PROVIDER || !Url::parse(server).is_ok_and(|url| is_cloud(&url)) {
            return Ok(None);
        }
        // El mismo orden con el que se usa: la referencia, el llavero y el entorno
        match configured_token(self.config_dir) {
            Some(value) if value.starts_with("op://") => Ok(Some(value)),
            Some(_) => Err(IssueError::new(Kind::TokenReference, "azure_devops.token")),
            None => Ok(self
                .store
                .get(KEYRING_NAME)
                .map_err(keyring_error)?
                .or_else(env_token)),
        }
    }
}

fn env_token() -> Option<String> {
    env("AZURE_DEVOPS_PAT").or_else(|| env("ADO_PAT"))
}

pub struct AzureDevOpsClient {
    token: Token,
    /// El servidor de la conexión; sin él (la implícita), la nube de Microsoft.
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
    pub fn configured(config_dir: &Path, connection: Option<&Connection>) -> Self {
        let token = match connection {
            Some(c) => match &c.secret {
                Some(secret) if secret.starts_with("op://") => Token::Reference(secret.clone()),
                Some(_) => Token::NotAReference("issues.connections"),
                None => Token::Stored {
                    keyring: c.keyring_name(),
                    connection: c.name.clone(),
                },
            },
            None => Token::choose(configured_token(config_dir), env_token()),
        };
        Self {
            token,
            server: connection.map(|c| c.server.clone()),
            op: env("SYNAPSE_OP_PATH").unwrap_or_else(|| "op".to_string()),
            store: SecretStore::from_env(),
        }
    }

    async fn token(&self) -> Result<String, IssueError> {
        let env = match &self.token {
            Token::NotAReference(key) => {
                return Err(IssueError::new(Kind::TokenReference, *key));
            }
            Token::Stored {
                keyring,
                connection,
            } => {
                let (store, name) = (self.store.clone(), keyring.clone());
                return tokio::task::spawn_blocking(move || store.get(&name))
                    .await
                    .map_err(|e| keyring_error(e.to_string()))?
                    .map_err(keyring_error)?
                    .ok_or_else(|| IssueError::new(Kind::ConnectionCredential, connection));
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
                        return Err(IssueError::new(
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
                .map_err(|e| IssueError::new(Kind::OnePassword, e)),
            (None, env) => Ok(env.unwrap_or_default()),
        }
    }

    fn project(&self, project_url: &str) -> Result<Project, IssueError> {
        let invalid = || IssueError::new(Kind::InvalidUrl, project_url);
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
        let allowed = match &self.server {
            Some(server) => connection::covers(server, project_url).is_some(),
            None => is_cloud(&url),
        };
        if !allowed {
            return Err(IssueError::new(Kind::HostNotAllowed, origin));
        }

        // Cuántos tramos forman la colección y cuál es el proyecto
        let (collection, name) = if is_cloud(&url) && host == "dev.azure.com" {
            (1, 1)
        } else if is_cloud(&url) {
            (0, 0)
        } else {
            // Servidor propio: el último tramo es el proyecto
            let last = segments.len().checked_sub(1).ok_or_else(invalid)?;
            (last, last)
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
    ) -> Result<Value, IssueError> {
        let token = self.token().await?;
        let failed = |e: reqwest::Error| IssueError::new(Kind::Request, e.to_string());
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
                .map_err(|_| IssueError::new(Kind::Auth, ""))?;
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

fn interpret(status: u16, text: &str) -> Result<Value, IssueError> {
    // 203: Azure DevOps responde así con la página de inicio de sesión
    if status == 401 || status == 203 || (300..400).contains(&status) {
        return Err(IssueError::new(Kind::Auth, format!("HTTP {status}")));
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
        return body.ok_or_else(|| IssueError::new(Kind::Auth, "HTML"));
    };
    let detail = if message.is_empty() {
        format!("HTTP {status}")
    } else {
        message.to_string()
    };
    Err(IssueError::new(kind, detail))
}

/// Una identidad llega como objeto o, en algunas respuestas, como texto; sin asignar, el campo falta.
fn person(value: &Value) -> Option<Person> {
    let name = value["displayName"].as_str().or_else(|| value.as_str())?;
    Some(Person {
        name: name.to_string(),
        id: value["uniqueName"].as_str().unwrap_or_default().to_string(),
    })
}

/// La categoría que declara Azure DevOps. «Resolved» sigue en curso hasta que se cierra; una que no
/// se conozca se deduce del nombre del estado. («Removed» no figura en la documentación de los estados, pero
/// la API lo declara.)
fn category(declared: &str, name: &str) -> StateCategory {
    match declared {
        "Proposed" => StateCategory::Todo,
        "InProgress" | "Resolved" => StateCategory::Doing,
        "Completed" => StateCategory::Done,
        "Removed" => StateCategory::Cancelled,
        _ => StateCategory::from_name(name),
    }
}

fn issue(value: &Value) -> Option<Issue> {
    let field = |name: &str| value["fields"][name].as_str().unwrap_or_default().to_string();
    let id = value["id"].as_u64()?;
    let state = field("System.State");
    let assignee = person(&value["fields"]["System.AssignedTo"]);
    Some(Issue {
        id: id.to_string(),
        key: format!("AB#{id}"),
        url: None,
        issue_type: field("System.WorkItemType"),
        title: field("System.Title"),
        description: description(value),
        state: IssueState {
            category: StateCategory::from_name(&state),
            name: state,
        },
        assignee,
        labels: Vec::new(),
        updated_at: Some(field("System.ChangedDate")).filter(|d| !d.is_empty()),
        revision: Some(value["rev"].as_u64()?.to_string()),
        extra: extra(value),
    })
}

/// Los campos que pueden llevar el texto principal, por preferencia: en un Bug, primero los pasos
/// para reproducirlo.
fn text_fields(value: &Value) -> [&'static str; 2] {
    if value["fields"]["System.WorkItemType"] == "Bug" {
        [REPRO_STEPS, DESCRIPTION]
    } else {
        [DESCRIPTION, REPRO_STEPS]
    }
}

/// El texto principal como Markdown.
fn description(value: &Value) -> Option<String> {
    text_fields(value).iter().find_map(|field| {
        let text = value["fields"][field].as_str()?;
        // Las organizaciones con campos en Markdown lo dicen aquí (no figura en la referencia)
        let format = value["multilineFieldsFormat"][field].as_str();
        Some(html_markdown::from_text(text, format)).filter(|t| !t.is_empty())
    })
}

/// El campo del que sale la descripción de un work item; sin texto en ninguno, el de su tipo.
fn text_field(value: &Value) -> &'static str {
    let order = text_fields(value);
    order
        .into_iter()
        .find(|field| {
            value["fields"][*field]
                .as_str()
                .is_some_and(|t| !html_markdown::from_text(t, None).is_empty())
        })
        .unwrap_or(order[0])
}

fn is_markdown(value: &Value, field: &str) -> bool {
    value["multilineFieldsFormat"][field]
        .as_str()
        .is_some_and(|f| f.eq_ignore_ascii_case("markdown"))
}

/// Lo que no cabe en el modelo común: cuántos comentarios tiene y si su descripción perdería
/// formato al reescribirla.
fn extra(value: &Value) -> BTreeMap<String, String> {
    let mut extra = BTreeMap::new();
    if let Some(count) = value["fields"][COMMENT_COUNT].as_u64() {
        extra.insert("comment_count".to_string(), count.to_string());
    }
    let field = text_field(value);
    if !is_markdown(value, field)
        && value["fields"][field]
            .as_str()
            .is_some_and(html_markdown::loses_format)
    {
        extra.insert("description_lossy".to_string(), "true".to_string());
    }
    extra
}

/// `2026-01-02T03:04:05.678Z` como `2026-01-02T03:04:05`, la forma de las fechas del tablero (UTC).
fn instant(date: &str) -> String {
    date.get(..19).unwrap_or(date).to_string()
}

fn comment_entry(value: &Value) -> Option<IssueEntry> {
    if value["isDeleted"] == true {
        return None;
    }
    // La referencia lo llama `id` y sus ejemplos, `commentId`
    let id = value["commentId"].as_u64().or_else(|| value["id"].as_u64())?;
    let text = match value["renderedText"].as_str().filter(|t| !t.is_empty()) {
        Some(html) => html_markdown::from_html(html),
        None => html_markdown::from_text(
            value["text"].as_str().unwrap_or_default(),
            value["format"].as_str(),
        ),
    };
    Some(IssueEntry {
        id: id.to_string(),
        kind: EntryKind::Comment,
        author: person(&value["createdBy"]),
        at: instant(value["createdDate"].as_str().unwrap_or_default()),
        text,
        changes: Vec::new(),
    })
}

/// Un valor del histórico en una línea.
fn shown(value: &Value) -> Option<String> {
    let text = match value {
        Value::Null => return None,
        Value::String(text) => text.clone(),
        Value::Object(_) => person(value)?.name,
        other => other.to_string(),
    };
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    Some(line.chars().take(200).collect::<String>()).filter(|l| !l.is_empty())
}

/// Un cambio del work item; `None` si no tocó nada que se guarde.
fn history_entry(value: &Value) -> Option<IssueEntry> {
    let fields = &value["fields"];
    let mut changes = Vec::new();
    let mut lines = Vec::new();
    for field in HISTORY_FIELDS {
        let change = &fields[field];
        let name = field.rsplit('.').next().unwrap_or(field);
        let long = field == DESCRIPTION || field == REPRO_STEPS;
        let (from, to) = (shown(&change["oldValue"]), shown(&change["newValue"]));
        if from == to {
            continue;
        }
        let (from, to) = if long { (None, None) } else { (from, to) };
        lines.push(match (&from, &to) {
            _ if long => format!("{name} changed"),
            (None, Some(to)) => format!("{name}: {to}"),
            (from, to) => format!(
                "{name}: {} → {}",
                from.as_deref().unwrap_or("—"),
                to.as_deref().unwrap_or("—")
            ),
        });
        changes.push(FieldChange {
            field: name.to_string(),
            from,
            to,
        });
    }
    if changes.is_empty() {
        return None;
    }
    // `revisedDate` es hasta cuándo valió la revisión (9999 en la última): la fecha del cambio va aparte
    let at = fields["System.ChangedDate"]["newValue"].as_str().or_else(|| {
        value["revisedDate"]
            .as_str()
            .filter(|d| !d.starts_with("9999"))
    });
    let author = &value["revisedBy"];
    Some(IssueEntry {
        id: format!("rev-{}", value["id"].as_u64()?),
        kind: EntryKind::History,
        author: person(author).or_else(|| person(&author["name"])),
        at: instant(at.unwrap_or_default()),
        text: lines.join("\n"),
        changes,
    })
}

fn expect_issue(value: &Value) -> Result<Issue, IssueError> {
    issue(value).ok_or_else(|| IssueError::new(Kind::Request, "respuesta sin work item"))
}

/// Los ids de Azure DevOps son números; cualquier otra cosa no es un work item suyo.
fn work_item_id(id: &str) -> Result<u64, IssueError> {
    id.parse()
        .map_err(|_| IssueError::new(Kind::NotFound, id))
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

impl AzureDevOpsClient {
    pub fn descriptor() -> ProviderDescriptor {
        let states = |names: &[&str]| names.iter().map(|n| n.to_string()).collect();
        ProviderDescriptor {
            id: PROVIDER.into(),
            name: "Azure DevOps".into(),
            query_language: "WIQL".into(),
            default_query: DEFAULT_QUERY.into(),
            default_issue_type: "Task".into(),
            default_states: [
                ("todo".to_string(), states(&["To Do", "New", "Proposed"])),
                (
                    "doing".to_string(),
                    states(&["In Progress", "Doing", "Active", "Committed", "Resolved"]),
                ),
                ("done".to_string(), states(&["Done", "Closed", "Completed"])),
            ]
            .into(),
            task_prefix: "ado".into(),
            scope_example: "https://dev.azure.com/organization/project".into(),
            filter_criteria: vec![
                Criterion::Type,
                Criterion::State,
                Criterion::Assignee,
                Criterion::Iteration,
                Criterion::Area,
                Criterion::Tag,
            ],
            connection_fields: vec![ConnectionField {
                id: "server".into(),
                required: true,
                example: "https://dev.azure.com/organization".into(),
            }],
            capabilities: Capabilities {
                create: true,
                comment: true,
                state_change: StateChange::Field,
                conflict_check: ConflictCheck::Revision,
            },
        }
    }
}

impl AzureDevOpsClient {
    /// El estado con el que un work item pasa a `new_type`: el suyo si ese tipo lo tiene; si no,
    /// uno de la misma categoría.
    async fn state_for_type(
        &self,
        project: &Project,
        id: u64,
        new_type: &str,
    ) -> Result<Value, IssueError> {
        let url = format!("{}/_apis/wit/workitemsbatch?{API}", project.collection);
        let body = json!({
            "ids": [id],
            "fields": ["System.WorkItemType", "System.State"],
            "errorPolicy": "Omit",
        });
        let response = self
            .send(Method::POST, &url, Some(("application/json", body)))
            .await?;
        let item =
            issue(&response["value"][0]).ok_or_else(|| IssueError::new(Kind::NotFound, id.to_string()))?;
        let states = self.type_states(project, new_type).await?;
        let mut found = states.iter().find(|s| s.name == item.state.name);
        if found.is_none() {
            let old = self.type_states(project, &item.issue_type).await?;
            let category = old
                .iter()
                .find(|s| s.name == item.state.name)
                .map_or(item.state.category, |s| s.category);
            found = states.iter().find(|s| s.category == category);
        }
        let state = found.or_else(|| states.first()).ok_or_else(|| {
            IssueError::new(Kind::Rule, format!("el tipo «{new_type}» no declara estados"))
        })?;
        Ok(set_field("System.State", &state.name))
    }

    /// Dónde y cómo escribir la descripción de un work item que ya existe.
    async fn description_patch(
        &self,
        project: &Project,
        id: u64,
        markdown: &str,
    ) -> Result<Value, IssueError> {
        let url = format!("{}/_apis/wit/workitemsbatch?{API}", project.collection);
        let body = json!({
            "ids": [id],
            "fields": ["System.WorkItemType", DESCRIPTION, REPRO_STEPS],
            "errorPolicy": "Omit",
        });
        let response = self
            .send(Method::POST, &url, Some(("application/json", body)))
            .await?;
        let item = &response["value"][0];
        if !item.is_object() {
            return Err(IssueError::new(Kind::NotFound, id.to_string()));
        }
        let field = text_field(item);
        let text = if is_markdown(item, field) {
            markdown.to_string()
        } else {
            html_markdown::to_html(markdown)
        };
        Ok(set_field(field, &text))
    }

    async fn comments(&self, project: &Project, id: u64) -> Result<Vec<IssueEntry>, IssueError> {
        let base = project.url(&format!("workItems/{id}/comments"));
        let mut found = Vec::new();
        let mut next: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut url = format!("{base}?$top={PAGE}&$expand=renderedText&{COMMENT_LIST_API}");
            if let Some(token) = &next {
                let token: String =
                    url::form_urlencoded::byte_serialize(token.as_bytes()).collect();
                url.push_str(&format!("&continuationToken={token}"));
            }
            let response = self.send(Method::GET, &url, None).await?;
            let page = response["comments"].as_array();
            found.extend(page.into_iter().flatten().filter_map(comment_entry));
            let token = response["continuationToken"]
                .as_str()
                .filter(|t| !t.is_empty())
                .map(str::to_string);
            // Un token repetido no avanza
            if token.is_none() || token == next {
                break;
            }
            next = token;
        }
        Ok(found)
    }

    async fn history(&self, project: &Project, id: u64) -> Result<Vec<IssueEntry>, IssueError> {
        let base = project.url(&format!("workItems/{id}/updates"));
        let mut found = Vec::new();
        let mut skip = 0;
        let mut last = 0;
        // La referencia no dice cuánto trae una página: se pide hasta que no llegue nada nuevo
        for _ in 0..MAX_PAGES {
            let url = format!("{base}?$top={PAGE}&$skip={skip}&{API}");
            let response = self.send(Method::GET, &url, None).await?;
            let page = response["value"].as_array().cloned().unwrap_or_default();
            // Llegan del más antiguo al más reciente
            let fresh: Vec<&Value> = page
                .iter()
                .filter(|u| u["id"].as_u64().is_some_and(|id| id > last))
                .collect();
            let Some(newest) = fresh.iter().filter_map(|u| u["id"].as_u64()).max() else {
                break;
            };
            last = newest;
            skip += page.len();
            found.extend(fresh.into_iter().filter_map(history_entry));
        }
        Ok(found)
    }

    /// Los estados de un tipo de work item, con su categoría.
    async fn type_states(
        &self,
        project: &Project,
        issue_type: &str,
    ) -> Result<Vec<IssueState>, IssueError> {
        let mut url = Url::parse(&project.url("workitemtypes"))
            .map_err(|_| IssueError::new(Kind::InvalidUrl, &project.name))?;
        if let Ok(mut path) = url.path_segments_mut() {
            path.push(issue_type).push("states");
        }
        url.set_query(Some(API));
        let response = self.send(Method::GET, url.as_str(), None).await?;
        let states = response["value"].as_array();
        Ok(states
            .map(|states| {
                states
                    .iter()
                    .filter_map(|state| {
                        let name = state["name"].as_str()?;
                        Some(IssueState {
                            category: category(state["category"].as_str().unwrap_or_default(), name),
                            name: name.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Pone a cada tarea la categoría que su proyecto declara para su estado. Si los estados no se
    /// pueden leer, queda la deducida del nombre: traer las tareas importa más.
    async fn declare_categories(&self, project: &Project, issues: &mut [Issue]) {
        let types: BTreeSet<String> = issues.iter().map(|i| i.issue_type.clone()).collect();
        for issue_type in types.iter().filter(|t| !t.is_empty()) {
            let Ok(states) = self.type_states(project, issue_type).await else {
                continue;
            };
            for issue in issues.iter_mut().filter(|i| &i.issue_type == issue_type) {
                if let Some(state) = states.iter().find(|s| s.name == issue.state.name) {
                    issue.state.category = state.category;
                }
            }
        }
    }
}

/// Un nodo de áreas o iteraciones: su ruta en WIQL (los nombres desde la raíz, que es el proyecto) y
/// las fechas de la iteración.
#[derive(Debug, PartialEq)]
struct Node {
    path: String,
    start: Option<String>,
    finish: Option<String>,
}

/// El árbol en preorden. La ruta se compone con los nombres: el formato del campo `path` de la
/// respuesta no está documentado.
fn nodes(node: &Value, parent: &str, found: &mut Vec<Node>) {
    let Some(name) = node["name"].as_str() else {
        return;
    };
    let path = if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}\\{name}")
    };
    // Las fechas llegan como `2026-01-01T00:00:00Z`
    let date = |key: &str| node["attributes"][key].as_str().and_then(|d| d.get(..10)).map(str::to_string);
    found.push(Node {
        path: path.clone(),
        start: date("startDate"),
        finish: date("finishDate"),
    });
    for child in node["children"].as_array().into_iter().flatten() {
        nodes(child, &path, found);
    }
}

fn current_iterations(nodes: &[Node], today: &str) -> Vec<String> {
    nodes
        .iter()
        .filter(|n| {
            n.start.as_deref().is_some_and(|s| s <= today)
                && n.finish.as_deref().is_some_and(|f| today <= f)
        })
        .map(|n| n.path.clone())
        .collect()
}

impl AzureDevOpsClient {
    /// `group` es `Areas` o `Iterations`.
    async fn classification(&self, project: &Project, group: &str) -> Result<Vec<Node>, IssueError> {
        let url = project.url(&format!("classificationnodes/{group}?$depth=14&{API}"));
        let response = self.send(Method::GET, &url, None).await?;
        let mut found = Vec::new();
        nodes(&response, "", &mut found);
        Ok(found)
    }

    async fn tags(&self, project: &Project) -> Result<Vec<String>, IssueError> {
        let url = project.url(&format!("tags?{API}"));
        let response = self.send(Method::GET, &url, None).await?;
        // La documentación muestra tanto `{value}` como esa respuesta dentro de una lista
        let wrapped = response.get(0).unwrap_or(&response);
        let tags = wrapped["value"].as_array().or_else(|| response.as_array());
        Ok(tags
            .into_iter()
            .flatten()
            .filter_map(|t| t["name"].as_str().map(str::to_string))
            .collect())
    }
}

impl IssueProvider for AzureDevOpsClient {
    fn describe(&self) -> ProviderDescriptor {
        Self::descriptor()
    }

    fn editable(&self, scope: &str) -> Vec<String> {
        let mut fields = vec!["title", "state", "assignee", "description", "comment"];
        // La referencia solo muestra el cambio de tipo en el servicio en la nube
        if Url::parse(scope.trim()).is_ok_and(|url| is_cloud(&url)) {
            fields.push("type");
        }
        fields.into_iter().map(str::to_string).collect()
    }

    /// `connectionData` de la organización o colección: es lo que usa el cliente oficial de
    /// Microsoft para saber quién entra (no figura en la referencia REST) y vale con cualquier
    /// alcance del token, a diferencia de la lista de proyectos.
    fn check<'a>(&'a self) -> ProviderFuture<'a, Account> {
        Box::pin(async move {
            let server = self
                .server
                .as_deref()
                .ok_or_else(|| IssueError::new(Kind::NoConnection, ""))?;
            let url = format!("{server}/_apis/connectionData");
            let response = self.send(Method::GET, &url, None).await?;
            let user = &response["authenticatedUser"];
            let name = ["customDisplayName", "providerDisplayName"]
                .iter()
                .find_map(|key| user[*key].as_str().filter(|v| !v.is_empty()));
            Ok(Account {
                name: name.map(str::to_string),
            })
        })
    }

    fn states<'a>(&'a self, scope: &'a str) -> ProviderFuture<'a, Vec<TypeStates>> {
        Box::pin(async move {
            let project = self.project(scope)?;
            let url = project.url(&format!("workitemtypes?{API}"));
            let response = self.send(Method::GET, &url, None).await?;
            let types = response["value"].as_array().cloned().unwrap_or_default();
            let mut found = Vec::new();
            for issue_type in types.iter().filter(|t| t["isDisabled"] != true) {
                if let Some(name) = issue_type["name"].as_str() {
                    found.push(TypeStates {
                        issue_type: name.to_string(),
                        states: self.type_states(&project, name).await?,
                    });
                }
            }
            Ok(found)
        })
    }

    fn values<'a>(
        &'a self,
        scope: &'a str,
        criterion: Criterion,
    ) -> ProviderFuture<'a, Vec<String>> {
        Box::pin(async move {
            let project = self.project(scope)?;
            let paths = |nodes: Vec<Node>| nodes.into_iter().map(|n| n.path);
            Ok(match criterion {
                // Listar el equipo pide un alcance que un token de work items no tiene: los demás
                // nombres los pone quien ya los vio o se escriben
                Criterion::Assignee => vec![ME.to_string()],
                Criterion::Type => {
                    let url = project.url(&format!("workitemtypes?{API}"));
                    let response = self.send(Method::GET, &url, None).await?;
                    response["value"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|t| t["isDisabled"] != true)
                        .filter_map(|t| t["name"].as_str().map(str::to_string))
                        .collect()
                }
                Criterion::State => {
                    let mut names: Vec<String> = Vec::new();
                    for state in self.states(scope).await?.into_iter().flat_map(|t| t.states) {
                        if !names.contains(&state.name) {
                            names.push(state.name);
                        }
                    }
                    names
                }
                Criterion::Iteration => std::iter::once(CURRENT.to_string())
                    .chain(paths(self.classification(&project, "Iterations").await?))
                    .collect(),
                Criterion::Area => paths(self.classification(&project, "Areas").await?).collect(),
                Criterion::Tag => self.tags(&project).await?,
            })
        })
    }

    fn filter_query<'a>(
        &'a self,
        scope: &'a str,
        filter: &'a IssueFilter,
        today: &'a str,
    ) -> ProviderFuture<'a, String> {
        Box::pin(async move {
            let project = self.project(scope)?;
            let current = if filter.iterations.iter().any(|i| i == CURRENT) {
                current_iterations(&self.classification(&project, "Iterations").await?, today)
            } else {
                Vec::new()
            };
            azure_devops_wiql::compile(filter, &current)
        })
    }

    fn search<'a>(&'a self, scope: &'a str, query: &'a str) -> ProviderFuture<'a, Vec<String>> {
        Box::pin(async move {
            let url = self.project(scope)?.url(&format!("wiql?{API}"));
            let body = ("application/json", json!({ "query": query }));
            let response = self.send(Method::POST, &url, Some(body)).await?;
            let items = response["workItems"].as_array();
            Ok(items
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|i| i["id"].as_u64())
                        .map(|id| id.to_string())
                        .collect()
                })
                .unwrap_or_default())
        })
    }

    fn get<'a>(&'a self, scope: &'a str, ids: &'a [String]) -> ProviderFuture<'a, Vec<Issue>> {
        Box::pin(async move {
            let project = self.project(scope)?;
            let url = format!("{}/_apis/wit/workitemsbatch?{API}", project.collection);
            let ids = ids
                .iter()
                .map(|id| work_item_id(id))
                .collect::<Result<Vec<_>, _>>()?;
            let mut found = Vec::new();
            for chunk in ids.chunks(BATCH) {
                let body = json!({
                    "ids": chunk,
                    "fields": FIELDS,
                    "errorPolicy": "Omit",
                });
                let response = self
                    .send(Method::POST, &url, Some(("application/json", body)))
                    .await?;
                // Con `Omit`, los que no se pueden leer faltan o llegan como `null`
                if let Some(items) = response["value"].as_array() {
                    found.extend(items.iter().filter_map(issue));
                }
            }
            self.declare_categories(&project, &mut found).await;
            Ok(found)
        })
    }

    fn update<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        revision: Option<&'a str>,
        changes: &'a IssueChanges,
    ) -> ProviderFuture<'a, Issue> {
        Box::pin(async move {
            let id = work_item_id(id)?;
            // Sin la revisión leída no hay forma de saber si se pisa un cambio ajeno
            let revision: u64 = revision
                .and_then(|r| r.parse().ok())
                .ok_or_else(|| IssueError::new(Kind::Conflict, "falta la revisión"))?;
            if changes.issue_type.is_some() && !self.editable(scope).contains(&"type".to_string()) {
                return Err(IssueError::new(
                    Kind::Rule,
                    "este servidor no admite cambiar el tipo",
                ));
            }
            let project = self.project(scope)?;
            let url = project.url(&format!("workitems/{id}?{API}"));
            let mut patch = vec![json!({"op": "test", "path": "/rev", "value": revision})];
            for (field, value) in [
                ("System.WorkItemType", &changes.issue_type),
                ("System.Title", &changes.title),
                ("System.State", &changes.state),
                // Vacío quita la asignación
                ("System.AssignedTo", &changes.assignee),
            ] {
                patch.extend(value.as_deref().map(|v| set_field(field, v)));
            }
            // La referencia cambia el tipo junto con un estado que el tipo nuevo admita
            if let (Some(new_type), None) = (&changes.issue_type, &changes.state) {
                patch.push(self.state_for_type(&project, id, new_type).await?);
            }
            if let Some(markdown) = &changes.description {
                patch.push(self.description_patch(&project, id, markdown).await?);
            }
            let body = ("application/json-patch+json", Value::Array(patch));
            expect_issue(&self.send(Method::PATCH, &url, Some(body)).await?)
        })
    }

    fn activity<'a>(&'a self, scope: &'a str, id: &'a str) -> ProviderFuture<'a, Vec<IssueEntry>> {
        Box::pin(async move {
            let id = work_item_id(id)?;
            let project = self.project(scope)?;
            let mut found = self.comments(&project, id).await?;
            found.extend(self.history(&project, id).await?);
            found.sort_by(|a, b| a.at.cmp(&b.at));
            Ok(found)
        })
    }

    fn create<'a>(&'a self, scope: &'a str, new: &'a NewIssue) -> ProviderFuture<'a, Issue> {
        Box::pin(async move {
            let base = self.project(scope)?.url("workitems");
            let mut url =
                Url::parse(&base).map_err(|_| IssueError::new(Kind::InvalidUrl, scope))?;
            if let Ok(mut path) = url.path_segments_mut() {
                path.push(&format!("${}", new.issue_type));
            }
            url.set_query(Some(API));
            let mut patch = vec![set_field("System.Title", &new.title)];
            patch.extend(
                new.assignee
                    .as_deref()
                    .map(|a| set_field("System.AssignedTo", a)),
            );
            if let Some(markdown) = new.description.as_deref().filter(|d| !d.trim().is_empty()) {
                let field = if new.issue_type == "Bug" {
                    REPRO_STEPS
                } else {
                    DESCRIPTION
                };
                patch.push(set_field(field, &html_markdown::to_html(markdown)));
            }
            let body = ("application/json-patch+json", Value::Array(patch));
            expect_issue(&self.send(Method::POST, url.as_str(), Some(body)).await?)
        })
    }

    fn add_comment<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        text: &'a str,
    ) -> ProviderFuture<'a, Option<String>> {
        Box::pin(async move {
            let id = work_item_id(id)?;
            let url = self
                .project(scope)?
                .url(&format!("workItems/{id}/comments?{COMMENTS_API}"));
            let body = ("application/json", json!({ "text": comment_html(text) }));
            let created = self.send(Method::POST, &url, Some(body)).await?;
            let id = created["commentId"].as_u64().or_else(|| created["id"].as_u64());
            Ok(id.map(|id| id.to_string()))
        })
    }

    fn update_comment<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        comment: &'a str,
        text: &'a str,
    ) -> ProviderFuture<'a, ()> {
        Box::pin(async move {
            let (id, comment) = (work_item_id(id)?, work_item_id(comment)?);
            let url = self.project(scope)?.url(&format!(
                "workItems/{id}/comments/{comment}?{COMMENT_LIST_API}"
            ));
            // Lo que se editó es Markdown: así se envía como se veía
            let body = ("application/json", json!({ "text": html_markdown::to_html(text) }));
            self.send(Method::PATCH, &url, Some(body)).await?;
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
            Token::NotAReference("azure_devops.token")
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
            AzureDevOpsClient::configured(dir.path(), None).token,
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
            .search("https://ejemplo.com/org/p", "SELECT")
            .await
            .unwrap_err();
        assert_eq!(error.kind, Kind::HostNotAllowed);
        assert!(!calls.exists());

        let ok = "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\n{}";
        let denied = "HTTP/1.1 401 Unauthorized\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
        for (reply, reads) in [(ok, 1), (ok, 1), (denied, 1), (ok, 2)] {
            let (origin, request) = serve(reply);
            let _ = with_op(&origin)
                .search(&format!("{origin}/org/p"), "SELECT")
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
            .search("https://dev.azure.com/org/Proyecto", "SELECT")
            .await
            .unwrap_err();
        assert_eq!(error.kind, Kind::OnePassword);
        assert!(error.detail.contains("/no/existe/op"));
    }

    #[test]
    fn sin_conexion_solo_admite_la_nube_de_azure_devops() {
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
    }

    #[test]
    fn con_una_conexion_solo_admite_lo_que_esta_en_su_servidor() {
        let own = client(Some("http://tfs.local:8080/tfs"));
        assert_eq!(
            own.project("http://tfs.local:8080/tfs/Coleccion/Proyecto")
                .unwrap(),
            Project {
                collection: "http://tfs.local:8080/tfs/Coleccion".into(),
                name: "Proyecto".into()
            }
        );
        let work = client(Some("https://dev.azure.com/org"));
        assert!(work.project("https://dev.azure.com/org/Proyecto").is_ok());
        for (azure, url) in [
            (&own, "http://tfs.local:9090/tfs/Coleccion/Proyecto"),
            (&own, "http://tfs.local:8080/otro/Proyecto"),
            (&own, "https://dev.azure.com/org/Proyecto"),
            (&work, "https://dev.azure.com/otra/Proyecto"),
        ] {
            assert_eq!(azure.project(url).unwrap_err().kind, Kind::HostNotAllowed, "{url}");
        }
    }

    #[tokio::test]
    async fn cada_conexion_usa_su_credencial_y_sin_ella_no_envia_nada() {
        let dir = tempfile::tempdir().unwrap();
        let (origin, request) = serve(
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\n{}",
        );
        let work = Connection {
            id: "trabajo".into(),
            provider: PROVIDER.into(),
            name: "Trabajo".into(),
            server: format!("{origin}/org"),
            user: None,
            secret: None,
        };
        let mut azure = AzureDevOpsClient::configured(dir.path(), Some(&work));
        azure.store = SecretStore::File(dir.path().to_path_buf());
        let scope = format!("{origin}/org/Proyecto");
        let error = azure.search(&scope, "SELECT").await.unwrap_err();
        assert_eq!(
            (error.kind, error.detail.as_str()),
            (Kind::ConnectionCredential, "Trabajo")
        );
        azure.store.set("issues-trabajo", "falso").unwrap();
        // La credencial global no sirve a una conexión
        azure.store.set(KEYRING_NAME, "global").unwrap();
        azure.search(&scope, "SELECT").await.unwrap();
        assert!(request.join().unwrap().contains("authorization: Basic OmZhbHNv"));
    }

    #[test]
    fn la_credencial_global_solo_se_entrega_para_la_nube() {
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::File(dir.path().join("llavero"));
        store.set(KEYRING_NAME, "global").unwrap();
        let global = AzureGlobalCredential {
            config_dir: dir.path(),
            store: &store,
        };
        let of = |provider, server| global.for_server(provider, server).unwrap();
        assert_eq!(of(PROVIDER, "https://dev.azure.com/org").as_deref(), Some("global"));
        assert_eq!(of(PROVIDER, "https://ajeno.test/org"), None);
        assert_eq!(of("otro", "https://dev.azure.com/org"), None);
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
        let changes = IssueChanges {
            state: Some("Done".into()),
            ..Default::default()
        };
        let item = client(Some(&origin))
            .update(&format!("{origin}/org/Proyecto"), "5", Some("2"), &changes)
            .await
            .unwrap();
        assert_eq!(
            (item.revision.as_deref(), item.state.name.as_str()),
            (Some("3"), "Done")
        );
        assert_eq!(item.state.category, StateCategory::Done);
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
            .search(&format!("{origin}/org/Proyecto"), "SELECT")
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
            .search("https://dev.azure.com/org/Proyecto", "SELECT")
            .await
            .unwrap_err();
        assert_eq!(error.kind, Kind::MissingToken);
    }

    #[test]
    fn traduce_un_work_item_al_modelo_comun() {
        let value = json!({"id": 7, "rev": 4, "fields": {
            "System.Title": "Publicar", "System.State": "In Progress", "System.WorkItemType": "Bug",
            "System.AssignedTo": {"displayName": "Ana", "uniqueName": "ana@ejemplo.test"},
            "System.ChangedDate": "2026-01-02T03:04:05Z",
        }});
        let found = issue(&value).unwrap();
        assert_eq!((found.id.as_str(), found.key.as_str()), ("7", "AB#7"));
        assert_eq!(found.issue_type, "Bug");
        assert_eq!(found.state.category, StateCategory::Doing);
        assert_eq!(found.assignee.unwrap().name, "Ana");
        assert_eq!(found.updated_at.as_deref(), Some("2026-01-02T03:04:05Z"));
        // Sin asignar ni fecha
        let bare = issue(&json!({"id": 8, "rev": 1, "fields": {}})).unwrap();
        assert_eq!((bare.assignee, bare.updated_at), (None, None));
        assert_eq!(person(&json!("Ana")).unwrap().name, "Ana");
    }

    /// Como `serve`, para varias peticiones seguidas: responde cada cuerpo JSON en orden.
    fn serve_json(bodies: Vec<&'static str>) -> (String, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            bodies
                .into_iter()
                .map(|body| {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut chunk = [0u8; 4096];
                    let mut received = Vec::new();
                    loop {
                        let n = stream.read(&mut chunk).unwrap();
                        received.extend_from_slice(&chunk[..n]);
                        let text = String::from_utf8_lossy(&received);
                        let complete = text.split_once("\r\n\r\n").is_some_and(|(head, sent)| {
                            let length = head
                                .to_lowercase()
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length:")?.trim().parse().ok())
                                .unwrap_or(0);
                            sent.len() >= length
                        });
                        if n == 0 || complete {
                            break;
                        }
                    }
                    let length = body.len();
                    let reply = format!(
                        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {length}\r\n\r\n{body}"
                    );
                    stream.write_all(reply.as_bytes()).unwrap();
                    String::from_utf8_lossy(&received).into_owned()
                })
                .collect()
        });
        (origin, handle)
    }

    #[test]
    fn la_categoria_declarada_manda_sobre_el_nombre() {
        assert_eq!(category("InProgress", "En espera"), StateCategory::Doing);
        assert_eq!(category("Resolved", "Hecho a medias"), StateCategory::Doing);
        assert_eq!(category("Completed", "En espera"), StateCategory::Done);
        assert_eq!(category("Proposed", "Done"), StateCategory::Todo);
        assert_eq!(category("Removed", "x"), StateCategory::Cancelled);
        assert_eq!(category("", "Closed"), StateCategory::Done);
    }

    #[tokio::test]
    async fn lo_traido_lleva_la_categoria_de_su_tipo() {
        let (origin, requests) = serve_json(vec![
            r#"{"value":[{"id":1,"rev":1,"fields":{"System.State":"En espera","System.WorkItemType":"Mi tipo"}}]}"#,
            r#"{"value":[{"name":"En espera","category":"InProgress"}]}"#,
        ]);
        let scope = format!("{origin}/org/Proyecto");
        let found = client(Some(&format!("{origin}/org")))
            .get(&scope, &["1".to_string()])
            .await
            .unwrap();
        assert_eq!(found[0].state.category, StateCategory::Doing);
        let requests = requests.join().unwrap();
        assert!(requests[1].starts_with(
            "GET /org/Proyecto/_apis/wit/workitemtypes/Mi%20tipo/states?api-version=7.1 "
        ));
    }

    #[tokio::test]
    async fn los_estados_se_leen_de_cada_tipo_activo() {
        let (origin, requests) = serve_json(vec![
            r#"{"value":[{"name":"Task"},{"name":"Viejo","isDisabled":true}]}"#,
            r#"{"value":[{"name":"New","category":"Proposed"}]}"#,
        ]);
        let found = client(Some(&format!("{origin}/org")))
            .states(&format!("{origin}/org/Proyecto"))
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].states[0].category, StateCategory::Todo);
        requests.join().unwrap();
    }

    #[tokio::test]
    async fn la_iteracion_en_curso_se_resuelve_por_fechas() {
        let tree = r#"{"name":"Proyecto","children":[
            {"name":"Sprint 1","attributes":{"startDate":"2026-01-01T00:00:00Z","finishDate":"2026-01-14T00:00:00Z"}},
            {"name":"Sprint 2","attributes":{"startDate":"2026-01-15T00:00:00Z","finishDate":"2026-01-28T00:00:00Z"}}]}"#;
        let (origin, requests) = serve_json(vec![tree, tree]);
        let azure = client(Some(&format!("{origin}/org")));
        let scope = format!("{origin}/org/Proyecto");
        let filter = IssueFilter {
            iterations: vec![CURRENT.into()],
            ..Default::default()
        };
        // El último día todavía cuenta
        let query = azure.filter_query(&scope, &filter, "2026-01-14").await.unwrap();
        assert!(query.contains("[System.IterationPath] UNDER 'Proyecto\\Sprint 1'"), "{query}");
        let values = azure.values(&scope, Criterion::Iteration).await.unwrap();
        assert_eq!(values, [CURRENT, "Proyecto", "Proyecto\\Sprint 1", "Proyecto\\Sprint 2"]);
        let requests = requests.join().unwrap();
        assert!(requests[0].starts_with(
            "GET /org/Proyecto/_apis/wit/classificationnodes/Iterations?$depth=14&api-version=7.1 "
        ));
    }

    #[tokio::test]
    async fn las_etiquetas_se_leen_con_o_sin_envoltorio() {
        let (origin, requests) = serve_json(vec![
            r#"{"count":1,"value":[{"name":"a"}]}"#,
            r#"[{"count":1,"value":[{"name":"b"}]}]"#,
            r#"[{"name":"c"}]"#,
        ]);
        let azure = client(Some(&format!("{origin}/org")));
        let scope = format!("{origin}/org/Proyecto");
        for expected in ["a", "b", "c"] {
            assert_eq!(azure.values(&scope, Criterion::Tag).await.unwrap(), [expected]);
        }
        requests.join().unwrap();
    }

    #[tokio::test]
    async fn probar_dice_con_que_cuenta_se_entra() {
        let (origin, request) = serve(
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 69\r\n\r\n{\"authenticatedUser\":{\"providerDisplayName\":\"Ana\",\"id\":\"1234567890\"}}",
        );
        let account = client(Some(&format!("{origin}/org")))
            .check()
            .await
            .unwrap();
        assert_eq!(account.name.as_deref(), Some("Ana"));
        assert!(
            request
                .join()
                .unwrap()
                .starts_with("GET /org/_apis/connectionData ")
        );
        // Sin conexión no hay nada que probar
        assert_eq!(client(None).check().await.unwrap_err().kind, Kind::NoConnection);
    }

    #[tokio::test]
    async fn sin_revision_o_con_un_id_ajeno_no_se_envia_nada() {
        let changes = IssueChanges::default();
        let scope = "https://dev.azure.com/org/Proyecto";
        let azure = client(None);
        let error = azure.update(scope, "5", None, &changes).await.unwrap_err();
        assert_eq!(error.kind, Kind::Conflict);
        let error = azure.add_comment(scope, "5/../6", "x").await.unwrap_err();
        assert_eq!(error.kind, Kind::NotFound);
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

    #[test]
    fn el_texto_principal_depende_del_tipo_y_llega_como_markdown() {
        let item = |kind: &str, extra: Value| {
            let mut value = json!({"id": 1, "rev": 1, "fields": {
                "System.WorkItemType": kind,
                "System.Description": "<div>la <b>descripción</b></div>",
                "Microsoft.VSTS.TCM.ReproSteps": "<ol><li>paso</li></ol>",
            }});
            if !extra.is_null() {
                value["multilineFieldsFormat"] = extra;
            }
            issue(&value).unwrap().description.unwrap()
        };
        assert_eq!(item("Task", Value::Null), "la **descripción**");
        assert_eq!(item("Bug", Value::Null), "1. paso");
        let markdown = json!({"System.Description": "markdown"});
        assert_eq!(item("Task", markdown), "&lt;div>la &lt;b>descripción&lt;/b>&lt;/div>");
        let empty = json!({"id": 1, "rev": 1, "fields": {"System.Description": "<div><br></div>"}});
        assert_eq!(issue(&empty).unwrap().description, None);
    }

    #[tokio::test]
    async fn la_actividad_une_comentarios_e_historico_por_fecha() {
        let (origin, requests) = serve_json(vec![
            r#"{"comments":[{"commentId":11,"text":"x","renderedText":"<div>Hola <b>equipo</b></div>","createdBy":{"displayName":"Ana"},"createdDate":"2026-01-03T10:00:00.5Z"}],"continuationToken":"a+b="}"#,
            r#"{"comments":[{"id":12,"text":"borrado","isDeleted":true},{"id":13,"text":"**md**","format":"markdown","createdDate":"2026-01-05T10:00:00Z"}]}"#,
            r#"{"value":[
                {"id":1,"revisedBy":{"displayName":"Luis"},"revisedDate":"2026-01-02T09:00:00Z","fields":{"System.State":{"newValue":"New"},"System.Rev":{"newValue":1},"System.ChangedDate":{"newValue":"2026-01-01T09:00:00Z"}}},
                {"id":2,"revisedDate":"2026-01-04T00:00:00Z","fields":{"System.Rev":{"newValue":2}}},
                {"id":3,"revisedBy":{"name":"Eva"},"revisedDate":"9999-01-01T00:00:00Z","fields":{"System.State":{"oldValue":"New","newValue":"Active"},"System.AssignedTo":{"newValue":{"displayName":"Ana"}},"System.Description":{"oldValue":"<p>a</p>","newValue":"<p>b</p>"},"System.ChangedDate":{"newValue":"2026-01-04T09:00:00.12Z"}}}
            ]}"#,
            r#"{"value":[]}"#,
        ]);
        let found = client(Some(&origin))
            .activity(&format!("{origin}/org/Proyecto"), "7")
            .await
            .unwrap();
        let lines: Vec<(&str, &str, &str)> = found
            .iter()
            .map(|e| (e.id.as_str(), e.at.as_str(), e.text.as_str()))
            .collect();
        assert_eq!(
            lines,
            [
                ("rev-1", "2026-01-01T09:00:00", "State: New"),
                ("11", "2026-01-03T10:00:00", "Hola **equipo**"),
                (
                    "rev-3",
                    "2026-01-04T09:00:00",
                    "State: New → Active\nAssignedTo: Ana\nDescription changed"
                ),
                ("13", "2026-01-05T10:00:00", "**md**"),
            ]
        );
        assert_eq!(found[1].kind, EntryKind::Comment);
        assert_eq!(found[2].author.as_ref().unwrap().name, "Eva");
        let requests = requests.join().unwrap();
        assert!(requests[0].starts_with("GET /org/Proyecto/_apis/wit/workItems/7/comments?$top=200&$expand=renderedText&api-version=7.1-preview.4 "), "{}", requests[0]);
        assert!(requests[1].contains("&continuationToken=a%2Bb%3D "), "{}", requests[1]);
        assert!(requests[2].starts_with("GET /org/Proyecto/_apis/wit/workItems/7/updates?$top=200&$skip=0&api-version=7.1 "), "{}", requests[2]);
    }

    /// Una página de cambios de estado con esos ids.
    fn updates(ids: std::ops::RangeInclusive<u64>) -> &'static str {
        let value: Vec<Value> = ids
            .map(|id| json!({"id": id, "fields": {"System.State": {"newValue": format!("E{id}")}}}))
            .collect();
        Box::leak(json!({ "value": value }).to_string().into_boxed_str())
    }

    async fn history_of(pages: Vec<&'static str>) -> (usize, Vec<String>) {
        let mut replies = vec![r#"{"comments":[]}"#];
        replies.extend(pages);
        let (origin, requests) = serve_json(replies);
        let found = client(Some(&origin))
            .activity(&format!("{origin}/org/Proyecto"), "7")
            .await
            .unwrap();
        let ids = found.into_iter().map(|e| e.id).collect();
        (requests.join().unwrap().len() - 1, ids)
    }

    #[tokio::test]
    async fn el_historico_sigue_aunque_la_pagina_venga_corta() {
        let pages = vec![updates(1..=100), updates(101..=130), r#"{"value":[]}"#];
        let (requests, ids) = history_of(pages).await;
        assert_eq!((requests, ids.len()), (3, 130));
        assert_eq!(ids.last().unwrap(), "rev-130");
    }

    #[tokio::test]
    async fn una_pagina_repetida_no_se_pide_sin_fin() {
        let (requests, ids) = history_of(vec![updates(1..=200), updates(1..=200)]).await;
        assert_eq!((requests, ids.len()), (2, 200));
    }

    #[tokio::test]
    async fn envia_los_campos_cambiados_y_la_descripcion_en_su_campo_como_html() {
        let (origin, requests) = serve_json(vec![
            r#"{"value":[{"id":5,"rev":2,"fields":{"System.WorkItemType":"Bug","Microsoft.VSTS.TCM.ReproSteps":"<p>antes</p>"}}]}"#,
            r#"{"id":5,"rev":3,"fields":{}}"#,
        ]);
        let changes = IssueChanges {
            assignee: Some(String::new()),
            description: Some("**ahora** <b>".into()),
            ..Default::default()
        };
        client(Some(&origin))
            .update(&format!("{origin}/org/Proyecto"), "5", Some("2"), &changes)
            .await
            .unwrap();
        let sent = requests.join().unwrap().pop().unwrap();
        assert!(sent.starts_with("PATCH /org/Proyecto/_apis/wit/workitems/5?"));
        assert!(sent.contains(r#""path":"/fields/System.AssignedTo","value":"""#), "{sent}");
        assert!(sent.contains(r#""path":"/fields/Microsoft.VSTS.TCM.ReproSteps","value":"<p><strong>ahora</strong> &lt;b&gt;</p>""#), "{sent}");
        assert!(!sent.contains("System.Description"));
    }

    #[tokio::test]
    async fn un_campo_en_markdown_se_envia_sin_convertir() {
        let (origin, requests) = serve_json(vec![
            r#"{"value":[{"id":5,"rev":2,"fields":{"System.Description":"antes"},"multilineFieldsFormat":{"System.Description":"markdown"}}]}"#,
            r#"{"id":5,"rev":3,"fields":{}}"#,
        ]);
        let changes = IssueChanges {
            description: Some("**ahora**".into()),
            ..Default::default()
        };
        client(Some(&origin))
            .update(&format!("{origin}/org/Proyecto"), "5", Some("2"), &changes)
            .await
            .unwrap();
        let sent = requests.join().unwrap().pop().unwrap();
        assert!(sent.contains(r#""path":"/fields/System.Description","value":"**ahora**""#), "{sent}");
    }

    #[tokio::test]
    async fn el_tipo_solo_se_cambia_en_la_nube() {
        let azure = client(Some("http://127.0.0.1:9"));
        let changes = IssueChanges {
            issue_type: Some("Bug".into()),
            ..Default::default()
        };
        // Se rechaza antes de pedir nada
        let error = azure
            .update("http://127.0.0.1:9/org/Proyecto", "5", Some("2"), &changes)
            .await
            .unwrap_err();
        assert_eq!(error.kind, Kind::Rule);
        let editable = |scope: &str| azure.editable(scope).contains(&"type".to_string());
        assert!(editable("https://dev.azure.com/org/Proyecto"));
        assert!(!editable("http://127.0.0.1:9/org/Proyecto"));
    }

    #[tokio::test]
    async fn edita_un_comentario_en_su_direccion() {
        let (origin, requests) = serve_json(vec![r#"{"commentId":11}"#]);
        client(Some(&origin))
            .update_comment(&format!("{origin}/org/Proyecto"), "7", "11", "a *b*")
            .await
            .unwrap();
        let sent = requests.join().unwrap().pop().unwrap();
        assert!(sent.starts_with("PATCH /org/Proyecto/_apis/wit/workItems/7/comments/11?api-version=7.1-preview.4 "), "{sent}");
        assert!(sent.contains(r#"{"text":"<p>a <em>b</em></p>"}"#), "{sent}");
    }

    #[tokio::test]
    async fn al_cambiar_de_tipo_elige_un_estado_que_el_nuevo_admita() {
        let (origin, requests) = serve_json(vec![
            r#"{"value":[{"id":5,"rev":2,"fields":{"System.WorkItemType":"Task","System.State":"Doing"}}]}"#,
            r#"{"value":[{"name":"New","category":"Proposed"},{"name":"Active","category":"InProgress"}]}"#,
            r#"{"value":[{"name":"Doing","category":"InProgress"}]}"#,
        ]);
        let azure = client(Some(&origin));
        let project = azure.project(&format!("{origin}/org/Proyecto")).unwrap();
        let patch = azure.state_for_type(&project, 5, "Bug").await.unwrap();
        assert_eq!((patch["path"].as_str(), patch["value"].as_str()), (Some("/fields/System.State"), Some("Active")));
        assert_eq!(requests.join().unwrap().len(), 3);
    }
}
