use crate::application::service::work_item_tracker::WorkItemTracker;
use crate::domain::work_item::work_item::WorkItem;
use crate::domain::work_item::work_item_error::WorkItemError;

/// Crea un work item.
pub struct CreateWorkItemUseCase<T: WorkItemTracker> {
    tracker: T,
}

impl<T: WorkItemTracker> CreateWorkItemUseCase<T> {
    pub fn new(tracker: T) -> Self {
        Self { tracker }
    }

    pub async fn execute(
        &self,
        project_url: &str,
        work_item_type: &str,
        title: &str,
        assigned_to: Option<&str>,
    ) -> Result<WorkItem, WorkItemError> {
        self.tracker
            .create(project_url, work_item_type, title, assigned_to)
            .await
    }
}
