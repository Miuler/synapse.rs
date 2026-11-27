use crate::application::dto::issue::connection_view::ConnectionView;
use crate::application::dto::issue::save_connection_command::SaveConnectionCommand;
use crate::application::service::issue_provider::IssueProvider;
use crate::application::use_cases::issue::add_issue_comment_use_case::AddIssueCommentUseCase;
use crate::application::use_cases::issue::check_connection_use_case::CheckConnectionUseCase;
use crate::application::use_cases::issue::create_issue_use_case::CreateIssueUseCase;
use crate::application::use_cases::issue::delete_connection_use_case::DeleteConnectionUseCase;
use crate::application::use_cases::issue::fetch_issues_use_case::{
    FetchIssuesUseCase, FetchedIssues,
};
use crate::application::use_cases::issue::list_connections_use_case::ListConnectionsUseCase;
use crate::application::use_cases::issue::save_connection_use_case::SaveConnectionUseCase;
use crate::application::use_cases::issue::update_issue_use_case::UpdateIssueUseCase;
use crate::domain::issue::issue::{Account, Issue, IssueChanges, NewIssue};
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind};
use crate::domain::issue::provider_descriptor::ProviderDescriptor;
use crate::infrastructure::repository::file::connection_repository_impl::ConnectionRepositoryImpl;
use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use crate::infrastructure::service::http::azure_devops_client::{
    self, AzureGlobalCredential, TokenSetting,
};
use crate::infrastructure::service::http::issue_providers;
use crate::infrastructure::service::os::secret_store::SecretStore;

fn connect(
    provider: &str,
    connection: Option<&str>,
    scope: &str,
) -> Result<Box<dyn IssueProvider>, IssueError> {
    issue_providers::connect(
        provider,
        connection,
        scope,
        &WorkspaceRegistryRepositoryImpl::resolve_config_dir(),
    )
}

/// Las conexiones tocan el llavero, que bloquea: fuera del hilo asíncrono.
async fn connections_task<T: Send + 'static>(
    task: impl FnOnce(std::path::PathBuf, SecretStore) -> Result<T, IssueError> + Send + 'static,
) -> Result<T, IssueError> {
    tauri::async_runtime::spawn_blocking(move || {
        task(
            WorkspaceRegistryRepositoryImpl::resolve_config_dir(),
            SecretStore::from_env(),
        )
    })
    .await
    .map_err(|e| IssueError::new(IssueErrorKind::Keyring, e.to_string()))?
}

/// Las conexiones configuradas, sin sus tokens.
#[tauri::command]
pub async fn issues_connections() -> Result<Vec<ConnectionView>, IssueError> {
    connections_task(|dir, store| {
        ListConnectionsUseCase::new(ConnectionRepositoryImpl::new(dir), store).execute()
    })
    .await
}

#[tauri::command]
pub async fn issues_save_connection(
    connection: SaveConnectionCommand,
) -> Result<ConnectionView, IssueError> {
    connections_task(move |dir, store| {
        let providers: Vec<String> = issue_providers::descriptors()
            .into_iter()
            .map(|p| p.id)
            .collect();
        SaveConnectionUseCase::new(
            ConnectionRepositoryImpl::new(dir.clone()),
            store.clone(),
            AzureGlobalCredential {
                config_dir: &dir,
                store: &store,
            },
        )
        .execute(connection, &providers)
    })
    .await
}

/// Con qué cuenta entra la credencial de una conexión guardada.
#[tauri::command]
pub async fn issues_check_connection(
    provider: String,
    connection: String,
) -> Result<Account, IssueError> {
    CheckConnectionUseCase::new(connect(&provider, Some(&connection), "")?)
        .execute()
        .await
}

#[tauri::command]
pub async fn issues_delete_connection(id: String) -> Result<(), IssueError> {
    connections_task(move |dir, store| {
        DeleteConnectionUseCase::new(ConnectionRepositoryImpl::new(dir), store).execute(&id)
    })
    .await
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
    provider: String,
    connection: Option<String>,
    scope: String,
    query: String,
    linked: Vec<String>,
) -> Result<FetchedIssues, IssueError> {
    FetchIssuesUseCase::new(connect(&provider, connection.as_deref(), &scope)?)
        .execute(&scope, &query, &linked)
        .await
}

#[tauri::command]
pub async fn issues_update(
    provider: String,
    connection: Option<String>,
    scope: String,
    id: String,
    revision: Option<String>,
    changes: IssueChanges,
) -> Result<Issue, IssueError> {
    UpdateIssueUseCase::new(connect(&provider, connection.as_deref(), &scope)?)
        .execute(&scope, &id, revision.as_deref(), &changes)
        .await
}

#[tauri::command]
pub async fn issues_create(
    provider: String,
    connection: Option<String>,
    scope: String,
    issue: NewIssue,
) -> Result<Issue, IssueError> {
    CreateIssueUseCase::new(connect(&provider, connection.as_deref(), &scope)?)
        .execute(&scope, &issue)
        .await
}

#[tauri::command]
pub async fn issues_add_comment(
    provider: String,
    connection: Option<String>,
    scope: String,
    id: String,
    text: String,
) -> Result<(), IssueError> {
    AddIssueCommentUseCase::new(connect(&provider, connection.as_deref(), &scope)?)
        .execute(&scope, &id, &text)
        .await
}
