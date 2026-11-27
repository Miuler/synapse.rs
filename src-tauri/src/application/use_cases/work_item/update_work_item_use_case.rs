use crate::application::service::work_item_tracker::WorkItemTracker;
use crate::domain::work_item::work_item::WorkItem;
use crate::domain::work_item::work_item_error::WorkItemError;

/// Cambia el título o el estado de un work item.
pub struct UpdateWorkItemUseCase<T: WorkItemTracker> {
    tracker: T,
}

impl<T: WorkItemTracker> UpdateWorkItemUseCase<T> {
    pub fn new(tracker: T) -> Self {
        Self { tracker }
    }

    pub async fn execute(
        &self,
        project_url: &str,
        id: u64,
        rev: u64,
        title: Option<&str>,
        state: Option<&str>,
    ) -> Result<WorkItem, WorkItemError> {
        self.tracker
            .update(project_url, id, rev, title, state)
            .await
    }
}
