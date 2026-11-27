//! El formato de carpeta de los tableros (`format: 2`): `Nombre.board.md` define el tablero y el
//! orden, y `Nombre.board/<id>.md` guarda cada tarea. Las claves y los encabezados son parte del
//! formato, en inglés.

pub mod board_error;
pub mod board_file;
pub mod board_repository;
pub mod protection;
pub mod task_file;
