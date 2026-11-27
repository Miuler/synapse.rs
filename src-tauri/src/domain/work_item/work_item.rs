use serde::Serialize;

/// Work item de un gestor de tareas externo (Azure DevOps).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkItem {
    pub id: u64,
    /// Revisión: una escritura solo se admite sobre la revisión que se leyó.
    pub rev: u64,
    pub title: String,
    pub state: String,
    pub work_item_type: String,
}
