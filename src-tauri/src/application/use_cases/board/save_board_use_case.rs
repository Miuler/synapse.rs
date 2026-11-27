use crate::domain::board::board_error::BoardError;
use crate::domain::board::board_file::BoardFile;
use crate::domain::board::board_repository::BoardRepository;
use crate::domain::board::task_file::TaskFile;
use std::path::Path;

/// Las escrituras de un tablero: su archivo y los de sus tareas.
pub struct SaveBoardUseCase<R: BoardRepository> {
    boards: R,
}

impl<R: BoardRepository> SaveBoardUseCase<R> {
    pub fn new(boards: R) -> Self {
        Self { boards }
    }

    pub fn board(
        &self,
        path: &Path,
        file: &BoardFile,
        expected: Option<&str>,
    ) -> Result<String, BoardError> {
        self.boards.write(path, file, expected)
    }

    pub fn task(
        &self,
        path: &Path,
        id: &str,
        task: &TaskFile,
        expected: Option<&str>,
    ) -> Result<String, BoardError> {
        self.boards.write_task(path, id, task, expected)
    }

    pub fn delete_task(&self, path: &Path, id: &str, expected: &str) -> Result<(), BoardError> {
        self.boards.delete_task(path, id, expected)
    }
}
