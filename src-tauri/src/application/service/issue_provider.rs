use crate::domain::issue::filter::{Criterion, IssueFilter};
use crate::domain::issue::issue::{Account, Issue, IssueChanges, IssueEntry, NewIssue, TypeStates};
use crate::domain::issue::issue_error::IssueError;
use crate::domain::issue::provider_descriptor::ProviderDescriptor;
use std::future::Future;
use std::pin::Pin;

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, IssueError>> + Send + 'a>>;

/// Puerto de un gestor de issues, ya vinculado a una conexión. `scope` es el ámbito dentro de ella
/// (proyecto o repositorio), en la forma que entienda el proveedor.
pub trait IssueProvider: Send + Sync {
    fn describe(&self) -> ProviderDescriptor;

    /// Comprueba la conexión y su credencial.
    fn check<'a>(&'a self) -> ProviderFuture<'a, Account>;

    /// Los tipos de tarea del ámbito con sus estados.
    fn states<'a>(&'a self, scope: &'a str) -> ProviderFuture<'a, Vec<TypeStates>>;

    /// Los valores que se pueden elegir en un criterio del filtro.
    fn values<'a>(
        &'a self,
        scope: &'a str,
        criterion: Criterion,
    ) -> ProviderFuture<'a, Vec<String>>;

    /// La consulta del proveedor para un filtro. `today` (`AAAA-MM-DD`) decide la iteración en curso.
    fn filter_query<'a>(
        &'a self,
        scope: &'a str,
        filter: &'a IssueFilter,
        today: &'a str,
    ) -> ProviderFuture<'a, String>;

    /// Ids que devuelve la consulta.
    fn search<'a>(&'a self, scope: &'a str, query: &'a str) -> ProviderFuture<'a, Vec<String>>;

    /// Las tareas que existen y se pueden leer entre las pedidas.
    fn get<'a>(&'a self, scope: &'a str, ids: &'a [String]) -> ProviderFuture<'a, Vec<Issue>>;

    /// Los comentarios y el histórico de una tarea, del más antiguo al más reciente.
    fn activity<'a>(&'a self, scope: &'a str, id: &'a str) -> ProviderFuture<'a, Vec<IssueEntry>>;

    fn create<'a>(&'a self, scope: &'a str, issue: &'a NewIssue) -> ProviderFuture<'a, Issue>;

    /// `revision` es la leída: si la tarea cambió desde entonces, falla sin escribir.
    fn update<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        revision: Option<&'a str>,
        changes: &'a IssueChanges,
    ) -> ProviderFuture<'a, Issue>;

    /// Devuelve el id del comentario creado, si el proveedor lo dice.
    fn add_comment<'a>(
        &'a self,
        scope: &'a str,
        id: &'a str,
        text: &'a str,
    ) -> ProviderFuture<'a, Option<String>>;
}
