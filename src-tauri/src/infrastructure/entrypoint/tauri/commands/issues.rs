use crate::application::service::issue_provider::IssueProvider;
use crate::application::use_cases::issue::add_issue_comment_use_case::AddIssueCommentUseCase;
use crate::application::use_cases::issue::create_issue_use_case::CreateIssueUseCase;
use crate::application::use_cases::issue::fetch_issues_use_case::{
    FetchIssuesUseCase, FetchedIssues,
};
use crate::application::use_cases::issue::update_issue_use_case::UpdateIssueUseCase;
use crate::domain::issue::issue::{Issue, IssueChanges, NewIssue};
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind};
use crate::domain::issue::provider_descriptor::ProviderDescriptor;
use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use crate::infrastructure::service::http::azure_devops_client::{self, TokenSetting};
use crate::infrastructure::service::http::issue_providers;
use crate::infrastructure::service::os::secret_store::SecretStore;

fn connect(connection: &str) -> Result<Box<dyn IssueProvider>, IssueError> {
    issue_providers::connect(
        connection,
        &WorkspaceRegistryRepositoryImpl::resolve_config_dir(),
    )
}

/// El llavero bloquea (y puede esperar a que se desbloquee): fuera del hilo asíncrono.
async fn token_task(value: Option<String>) -> Result<TokenSetting, IssueError> {
    tauri::async_runtime::spawn_blocking(move || {
        let config_dir = WorkspaceRegistryRepositoryImpl::resolve_config_dir();
        let store = SecretStore::from_env();
        if let Some(value) = value {
            azure_devops_client::save_token(&config_dir, &store, &value)?;
        }
        azure_devops_client::token_setting(&config_dir, &store)
    })
    .await
    .map_err(|e| IssueError::new(IssueErrorKind::Keyring, e.to_string()))?
}

/// De dónde sale el token (sin su valor).
#[tauri::command]
pub async fn get_azure_token_setting() -> Result<TokenSetting, IssueError> {
    token_task(None).await
}

/// Guarda una referencia `op://` o un token; vacío, lo quita.
#[tauri::command]
pub async fn set_azure_token(value: String) -> Result<TokenSetting, IssueError> {
    token_task(Some(value)).await
}

#[tauri::command]
pub fn issues_providers() -> Vec<ProviderDescriptor> {
    issue_providers::descriptors()
}

#[tauri::command]
pub async fn issues_fetch(
    connection: String,
    scope: String,
    query: String,
    linked: Vec<String>,
) -> Result<FetchedIssues, IssueError> {
    FetchIssuesUseCase::new(connect(&connection)?)
        .execute(&scope, &query, &linked)
        .await
}

#[tauri::command]
pub async fn issues_update(
    connection: String,
    scope: String,
    id: String,
    revision: Option<String>,
    changes: IssueChanges,
) -> Result<Issue, IssueError> {
    UpdateIssueUseCase::new(connect(&connection)?)
        .execute(&scope, &id, revision.as_deref(), &changes)
        .await
}

#[tauri::command]
pub async fn issues_create(
    connection: String,
    scope: String,
    issue: NewIssue,
) -> Result<Issue, IssueError> {
    CreateIssueUseCase::new(connect(&connection)?)
        .execute(&scope, &issue)
        .await
}

#[tauri::command]
pub async fn issues_add_comment(
    connection: String,
    scope: String,
    id: String,
    text: String,
) -> Result<(), IssueError> {
    AddIssueCommentUseCase::new(connect(&connection)?)
        .execute(&scope, &id, &text)
        .await
}
