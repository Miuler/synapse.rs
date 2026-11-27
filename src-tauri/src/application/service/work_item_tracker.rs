use crate::domain::work_item::work_item::WorkItem;
use crate::domain::work_item::work_item_error::WorkItemError;
use std::future::Future;
use std::pin::Pin;

pub type TrackerFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, WorkItemError>> + Send + 'a>>;

/// Puerto de un gestor de work items. `project_url` identifica el proyecto.
pub trait WorkItemTracker: Send + Sync {
    /// Ids que devuelve la consulta.
    fn query<'a>(&'a self, project_url: &'a str, query: &'a str) -> TrackerFuture<'a, Vec<u64>>;

    /// Los work items que existen y se pueden leer entre los pedidos.
    fn work_items<'a>(
        &'a self,
        project_url: &'a str,
        ids: &'a [u64],
    ) -> TrackerFuture<'a, Vec<WorkItem>>;

    fn update<'a>(
        &'a self,
        project_url: &'a str,
        id: u64,
        rev: u64,
        title: Option<&'a str>,
        state: Option<&'a str>,
    ) -> TrackerFuture<'a, WorkItem>;

    fn create<'a>(
        &'a self,
        project_url: &'a str,
        work_item_type: &'a str,
        title: &'a str,
        assigned_to: Option<&'a str>,
    ) -> TrackerFuture<'a, WorkItem>;

    fn add_comment<'a>(&'a self, project_url: &'a str, id: u64, text: &'a str)
    -> TrackerFuture<'a, ()>;
}
