use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardErrorKind {
    /// El archivo no es un tablero en formato de carpeta.
    NotFolderBoard,
    /// Un bloque del tablero que se interpreta está mal escrito: no se reescribe para no perderlo.
    Frontmatter,
    /// La línea `sync` de una tarea está mal escrita.
    Sync,
    /// Un identificador de tarea que no sirve como nombre de archivo.
    TaskId,
    /// La ruta no es la de un tablero de la bóveda.
    Path,
    /// El archivo cambió desde que se leyó, o ya existe uno que se iba a crear.
    Conflict,
    NotFound,
    Io,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoardError {
    pub kind: BoardErrorKind,
    pub detail: String,
}

impl BoardError {
    pub fn new(kind: BoardErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}
