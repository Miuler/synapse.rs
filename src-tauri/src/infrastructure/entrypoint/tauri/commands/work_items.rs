use crate::application::use_cases::work_item::add_work_item_comment_use_case::AddWorkItemCommentUseCase;
use crate::application::use_cases::work_item::create_work_item_use_case::CreateWorkItemUseCase;
use crate::application::use_cases::work_item::fetch_work_items_use_case::{
    FetchWorkItemsUseCase, FetchedWorkItems,
};
use crate::application::use_cases::work_item::update_work_item_use_case::UpdateWorkItemUseCase;
use crate::domain::work_item::work_item::WorkItem;
use crate::domain::work_item::work_item_error::WorkItemError;
use crate::infrastructure::service::http::azure_devops_client::AzureDevOpsClient;

#[tauri::command]
pub async fn azure_fetch_work_items(
    project_url: String,
    query: String,
    linked: Vec<u64>,
) -> Result<FetchedWorkItems, WorkItemError> {
    FetchWorkItemsUseCase::new(AzureDevOpsClient::from_env())
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
    UpdateWorkItemUseCase::new(AzureDevOpsClient::from_env())
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
    CreateWorkItemUseCase::new(AzureDevOpsClient::from_env())
        .execute(&project_url, &work_item_type, &title, assigned_to.as_deref())
        .await
}

#[tauri::command]
pub async fn azure_add_work_item_comment(
    project_url: String,
    id: u64,
    text: String,
) -> Result<(), WorkItemError> {
    AddWorkItemCommentUseCase::new(AzureDevOpsClient::from_env())
        .execute(&project_url, id, &text)
        .await
}
