use crate::application::service::work_item_tracker::WorkItemTracker;
use crate::domain::work_item::work_item_error::WorkItemError;

/// Añade un comentario a un work item.
pub struct AddWorkItemCommentUseCase<T: WorkItemTracker> {
    tracker: T,
}

impl<T: WorkItemTracker> AddWorkItemCommentUseCase<T> {
    pub fn new(tracker: T) -> Self {
        Self { tracker }
    }

    pub async fn execute(
        &self,
        project_url: &str,
        id: u64,
        text: &str,
    ) -> Result<(), WorkItemError> {
        self.tracker.add_comment(project_url, id, text).await
    }
}
