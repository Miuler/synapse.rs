//! La comprobación por la que pasan las escrituras de notas y de elementos de la bóveda. Las órdenes
//! del tablero escriben por su propio repositorio.

use crate::domain::board::protection;
use std::path::Path;

/// El archivo del tablero al que pertenece `path`, si está en la carpeta de uno.
pub fn owner(path: &Path) -> Option<String> {
    protection::guards(path)
        .find(|board| board.is_file())
        .map(|board| {
            board
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
}

pub fn ensure_writable(path: &Path) -> Result<(), String> {
    match owner(path) {
        None => Ok(()),
        Some(board) => Err(format!(
            "{} ({board}): belongs to that board and is changed from it",
            path.file_name().unwrap_or_default().to_string_lossy()
        )),
    }
}
