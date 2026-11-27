//! Fusión de otra rama en la actual: arrancarla, resolver sus conflictos y confirmarla o cancelarla.

use super::history_change::UndoPoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MergeAction {
    /// Fusiona `target` (rama, tag o commit) en la rama actual.
    Start { target: String },
    /// Da por resuelto un archivo en conflicto y lo prepara.
    Resolve { path: String, resolution: Resolution },
    /// Crea el commit de la fusión; sin `message` se usa el que propone git.
    Continue { message: Option<String> },
    Abort,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resolution {
    /// El texto resultante de resolver a mano.
    Content { content: String },
    /// La versión de la rama actual (si allí no existe, el archivo se borra).
    Ours,
    /// La versión de lo que se está fusionando (si allí no existe, el archivo se borra).
    Theirs,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MergeOutcome {
    /// Terminada: `undo` devuelve la rama a como estaba.
    Merged { undo: UndoPoint },
    UpToDate,
    /// Quedó a medias: hay archivos en conflicto que resolver.
    Conflicts,
    Resolved,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MergeConflict {
    /// Ruta relativa a la raíz del repositorio.
    pub path: String,
    /// Existe en el antecesor común.
    pub base: bool,
    /// Existe en la rama actual.
    pub ours: bool,
    /// Existe en lo que se está fusionando.
    pub theirs: bool,
    /// No se puede fusionar por líneas (binario, enlace o submódulo): solo elegir una versión.
    pub binary: bool,
}

/// Una fusión sin terminar, leída del repositorio.
#[derive(Debug, Clone, Serialize)]
pub struct MergeState {
    pub ours: String,
    pub theirs: String,
    /// Mensaje propuesto para el commit.
    pub message: String,
    pub conflicts: Vec<MergeConflict>,
}

/// Las tres versiones de un archivo en conflicto; `None` si en esa no existe.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ConflictVersions {
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub binary: bool,
}
