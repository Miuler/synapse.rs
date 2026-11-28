//! Un tablero de carpeta en la bóveda. Las rutas son absolutas y apuntan al `.board.md`; sus tareas
//! están en la carpeta del mismo nombre sin `.md`.

use crate::domain::board::board_error::BoardError;
use crate::domain::board::board_file::BoardFile;
use crate::domain::board::task_file::{ActivityEntry, TaskFile, TaskSync};
use serde::Serialize;
use std::path::Path;

/// Lo de una tarea que necesitan las vistas. No lleva la descripción ni la actividad, así que no
/// sirve para escribirla: para eso se lee entera.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCard {
    pub id: String,
    /// Del archivo: una escritura solo se admite sobre lo que se leyó.
    pub hash: String,
    pub title: String,
    pub status: String,
    #[serde(rename = "type")]
    pub issue_type: Option<String>,
    pub assignee: Option<String>,
    pub key: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<String>,
    pub severity: Option<String>,
    pub iteration: Option<String>,
    pub area: Option<String>,
    pub parent: Option<String>,
    pub estimate: Option<String>,
    pub due_date: Option<String>,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub changed_by: Option<String>,
    pub original_estimate: Option<String>,
    pub remaining_work: Option<String>,
    pub completed_work: Option<String>,
    pub board_column: Option<String>,
    pub board_lane: Option<String>,
    pub start: String,
    pub duration: u32,
    pub depends_on: String,
    pub progress: u8,
    pub sync: Option<TaskSync>,
    pub extra: Vec<(String, String)>,
    /// Sus comentarios (el histórico no).
    pub comments: Vec<ActivityEntry>,
}

/// Un archivo de la carpeta que no se pudo leer como tarea.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaskProblem {
    pub file: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoardSnapshot {
    pub board: BoardFile,
    pub hash: String,
    /// En el orden del tablero; las que no figuren en él, al final.
    pub tasks: Vec<TaskCard>,
    pub problems: Vec<TaskProblem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredTask {
    pub task: TaskFile,
    pub hash: String,
}

/// En las escrituras, `expected` es el hash leído; sin él, el archivo no debe existir.
pub trait BoardRepository: Send + Sync {
    fn read(&self, board: &Path) -> Result<BoardSnapshot, BoardError>;

    fn write(
        &self,
        board: &Path,
        file: &BoardFile,
        expected: Option<&str>,
    ) -> Result<String, BoardError>;

    fn read_task(&self, board: &Path, id: &str) -> Result<StoredTask, BoardError>;

    fn write_task(
        &self,
        board: &Path,
        id: &str,
        task: &TaskFile,
        expected: Option<&str>,
    ) -> Result<String, BoardError>;

    fn delete_task(&self, board: &Path, id: &str, expected: &str) -> Result<(), BoardError>;

    /// Pasa un tablero con tabla al formato de carpeta, dejando al lado una copia del original.
    /// `expected` es el contenido que se vio: si el archivo ya es otro, no se toca nada.
    fn convert(&self, board: &Path, expected: &str) -> Result<(), BoardError>;
}
