use crate::application::use_cases::work_item::add_work_item_comment_use_case::AddWorkItemCommentUseCase;
use crate::application::use_cases::work_item::create_work_item_use_case::CreateWorkItemUseCase;
use crate::application::use_cases::work_item::fetch_work_items_use_case::{
    FetchWorkItemsUseCase, FetchedWorkItems,
};
use crate::application::use_cases::work_item::update_work_item_use_case::UpdateWorkItemUseCase;
use crate::domain::work_item::work_item::WorkItem;
use crate::domain::work_item::work_item_error::{WorkItemError, WorkItemErrorKind};
use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use crate::infrastructure::service::http::azure_devops_client::{
    self, AzureDevOpsClient, TokenSetting,
};
use crate::infrastructure::service::os::secret_store::SecretStore;

fn client() -> AzureDevOpsClient {
    AzureDevOpsClient::configured(&WorkspaceRegistryRepositoryImpl::resolve_config_dir())
}

/// El llavero bloquea (y puede esperar a que se desbloquee): fuera del hilo asíncrono.
async fn token_task(value: Option<String>) -> Result<TokenSetting, WorkItemError> {
    tauri::async_runtime::spawn_blocking(move || {
        let config_dir = WorkspaceRegistryRepositoryImpl::resolve_config_dir();
        let store = SecretStore::from_env();
        if let Some(value) = value {
            azure_devops_client::save_token(&config_dir, &store, &value)?;
        }
        azure_devops_client::token_setting(&config_dir, &store)
    })
    .await
    .map_err(|e| WorkItemError::new(WorkItemErrorKind::Keyring, e.to_string()))?
}

/// De dónde sale el token (sin su valor).
#[tauri::command]
pub async fn get_azure_token_setting() -> Result<TokenSetting, WorkItemError> {
    token_task(None).await
}

/// Guarda una referencia `op://` o un token; vacío, lo quita.
#[tauri::command]
pub async fn set_azure_token(value: String) -> Result<TokenSetting, WorkItemError> {
    token_task(Some(value)).await
}

#[tauri::command]
pub async fn azure_fetch_work_items(
    project_url: String,
    query: String,
    linked: Vec<u64>,
) -> Result<FetchedWorkItems, WorkItemError> {
    FetchWorkItemsUseCase::new(client())
        .execute(&project_url, &query, &linked)
        .await
}

#[tauri::command]
pub async fn azure_update_work_item(
    project_url: String,
    id: u64,
    rev: u64,
    title: Option<String>,
    state: Option<String>,
) -> Result<WorkItem, WorkItemError> {
    UpdateWorkItemUseCase::new(client())
        .execute(&project_url, id, rev, title.as_deref(), state.as_deref())
        .await
}

#[tauri::command]
pub async fn azure_create_work_item(
    project_url: String,
    work_item_type: String,
    title: String,
    assigned_to: Option<String>,
) -> Result<WorkItem, WorkItemError> {
    CreateWorkItemUseCase::new(client())
        .execute(&project_url, &work_item_type, &title, assigned_to.as_deref())
        .await
}

#[tauri::command]
pub async fn azure_add_work_item_comment(
    project_url: String,
    id: u64,
    text: String,
) -> Result<(), WorkItemError> {
    AddWorkItemCommentUseCase::new(client())
        .execute(&project_url, id, &text)
        .await
}
