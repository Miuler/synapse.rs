use crate::application::service::issue_provider::IssueProvider;
use crate::domain::issue::issue::Issue;
use crate::domain::issue::issue_error::IssueError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FetchedIssues {
    pub items: Vec<Issue>,
    /// Ids pedidos: los que falten en `items` ya no existen o no se pueden leer.
    pub requested: Vec<String>,
    /// Los que devolvió la consulta: una vinculada que no esté quedó fuera de ella.
    pub matched: Vec<String>,
    /// Lo que el proveedor admite cambiar en ese ámbito.
    pub editable: Vec<String>,
}

/// Las tareas de la consulta y las que ya estaban vinculadas.
pub struct FetchIssuesUseCase {
    provider: Box<dyn IssueProvider>,
}

impl FetchIssuesUseCase {
    pub fn new(provider: Box<dyn IssueProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(
        &self,
        scope: &str,
        query: &str,
        linked: &[String],
    ) -> Result<FetchedIssues, IssueError> {
        // Sin consulta, solo las vinculadas
        let matched = if query.is_empty() {
            Vec::new()
        } else {
            self.provider.search(scope, query).await?
        };
        let mut requested = matched.clone();
        for id in linked {
            if !requested.contains(id) {
                requested.push(id.clone());
            }
        }
        let items = self.provider.get(scope, &requested).await?;
        Ok(FetchedIssues {
            items,
            requested,
            matched,
            editable: self.provider.editable(scope),
        })
    }
}
