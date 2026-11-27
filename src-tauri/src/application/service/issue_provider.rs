use crate::domain::issue::issue::{Issue, IssueChanges, NewIssue};
use crate::domain::issue::issue_error::IssueError;
use crate::domain::issue::provider_descriptor::ProviderDescriptor;
use std::future::Future;
use std::pin::Pin;

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, IssueError>> + Send + 'a>>;

/// Puerto de un gestor de issues, ya vinculado a una conexión. `scope` es el ámbito dentro de ella
/// (proyecto o repositorio), en la forma que entienda el proveedor.
pub trait IssueProvider: Send + Sync {
    fn describe(&self) -> ProviderDescriptor;

    /// Ids que devuelve la consulta.
    fn search<'a>(&'a self, scope: &'a str, query: &'a str) -> ProviderFuture<'a, Vec<String>>;

    /// Las tareas que existen y se pueden leer entre las pedidas.
    fn get<'a>(&'a self, scope: &'a str, ids: &'a [String]) -> ProviderFuture<'a, Vec<Issue>>;

    fn create<'a>(&'a self, scope: &'a str, issue: &'a NewIssue) -> ProviderFuture<'a, Issue>;

    /// `revision` es la leída: si la tarea cambió desde entonces, falla sin escribir.
    fn update<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        revision: Option<&'a str>,
        changes: &'a IssueChanges,
    ) -> ProviderFuture<'a, Issue>;

    fn add_comment<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        text: &'a str,
    ) -> ProviderFuture<'a, ()>;
}
