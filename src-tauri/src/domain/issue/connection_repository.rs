use crate::domain::issue::connection::Connection;
use crate::domain::issue::issue_error::IssueError;

/// Las conexiones configuradas, sin sus tokens.
pub trait ConnectionRepository: Send + Sync {
    fn list(&self) -> Result<Vec<Connection>, IssueError>;
    fn save(&self, connections: &[Connection]) -> Result<(), IssueError>;
}
