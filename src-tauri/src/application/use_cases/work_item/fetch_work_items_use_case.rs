use crate::application::service::work_item_tracker::WorkItemTracker;
use crate::domain::work_item::work_item::WorkItem;
use crate::domain::work_item::work_item_error::WorkItemError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FetchedWorkItems {
    pub items: Vec<WorkItem>,
    /// Ids pedidos: los que falten en `items` ya no existen o no se pueden leer.
    pub requested: Vec<u64>,
}

/// Los work items de la consulta y los que ya estaban vinculados.
pub struct FetchWorkItemsUseCase<T: WorkItemTracker> {
    tracker: T,
}

impl<T: WorkItemTracker> FetchWorkItemsUseCase<T> {
    pub fn new(tracker: T) -> Self {
        Self { tracker }
    }

    pub async fn execute(
        &self,
        project_url: &str,
        query: &str,
        linked: &[u64],
    ) -> Result<FetchedWorkItems, WorkItemError> {
        // Sin consulta, solo los vinculados
        let mut requested = if query.is_empty() {
            Vec::new()
        } else {
            self.tracker.query(project_url, query).await?
        };
        for id in linked {
            if !requested.contains(id) {
                requested.push(*id);
            }
        }
        let items = self.tracker.work_items(project_url, &requested).await?;
        Ok(FetchedWorkItems { items, requested })
    }
}
