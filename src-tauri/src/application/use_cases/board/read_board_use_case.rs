use crate::domain::board::board_error::BoardError;
use crate::domain::board::board_repository::{BoardRepository, BoardSnapshot, StoredTask};
use std::path::Path;

/// El tablero con las tarjetas de sus tareas, o una tarea entera.
pub struct ReadBoardUseCase<R: BoardRepository> {
    boards: R,
}

impl<R: BoardRepository> ReadBoardUseCase<R> {
    pub fn new(boards: R) -> Self {
        Self { boards }
    }

    pub fn execute(&self, board: &Path) -> Result<BoardSnapshot, BoardError> {
        self.boards.read(board)
    }

    pub fn task(&self, board: &Path, id: &str) -> Result<StoredTask, BoardError> {
        self.boards.read_task(board, id)
    }
}
