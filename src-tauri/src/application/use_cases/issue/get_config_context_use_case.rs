use crate::application::dto::issue::source_draft::{QueryMode, SourceDraft};
use crate::domain::issue::connection;
use crate::domain::issue::connection_repository::ConnectionRepository;
use crate::domain::issue::filter::IssueFilter;
use crate::domain::issue::issue_error::{IssueError, IssueErrorKind};
use crate::domain::issue::provider_descriptor::ProviderDescriptor;
use serde::Serialize;

/// De una conexión, solo esto: nada de su credencial.
#[derive(Debug, Serialize)]
pub struct ContextConnection {
    pub name: String,
    pub provider: String,
    pub server: String,
}

#[derive(Debug, Serialize)]
pub struct ContextColumn {
    pub id: String,
    pub title: String,
    /// Estados que valen para la columna: los configurados o los habituales del proveedor.
    pub states: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfigContext {
    pub view: &'static str,
    pub provider: String,
    pub query_language: String,
    /// Sin ella y sin problema, el proveedor no tiene conexiones y se usa la credencial global.
    pub connection: Option<ContextConnection>,
    #[serde(skip)]
    pub connection_problem: Option<IssueErrorKind>,
    pub scope: String,
    pub mode: QueryMode,
    pub query: String,
    pub filter: IssueFilter,
    pub columns: Vec<ContextColumn>,
}

pub struct GetConfigContextUseCase<R: ConnectionRepository> {
    connections: R,
}

impl<R: ConnectionRepository> GetConfigContextUseCase<R> {
    pub fn new(connections: R) -> Self {
        Self { connections }
    }

    pub fn execute(
        &self,
        draft: &SourceDraft,
        provider: &ProviderDescriptor,
    ) -> Result<ConfigContext, IssueError> {
        let connections = self.connections.list()?;
        let wanted = Some(draft.connection.as_str()).filter(|id| !id.is_empty());
        let chosen = connection::choose(&connections, &draft.provider, wanted, &draft.scope);
        let columns = draft
            .columns
            .iter()
            .map(|column| ContextColumn {
                id: column.id.clone(),
                title: column.title.clone(),
                states: draft
                    .states
                    .get(&column.id)
                    .or_else(|| provider.default_states.get(&column.id))
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect();
        let (connection, connection_problem) = match chosen {
            Ok(found) => (
                found.map(|c| ContextConnection {
                    name: c.name.clone(),
                    provider: c.provider.clone(),
                    server: c.server.clone(),
                }),
                None,
            ),
            Err(error) => (None, Some(error.kind)),
        };
        Ok(ConfigContext {
            view: "board_source",
            provider: provider.name.clone(),
            query_language: provider.query_language.clone(),
            connection,
            connection_problem,
            scope: draft.scope.clone(),
            mode: draft.mode,
            query: draft.query.clone(),
            filter: draft.filter.clone(),
            columns,
        })
    }
}
