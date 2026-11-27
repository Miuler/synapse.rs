//! Cambios en la historia de la rama actual: squash, revert y reset, y cómo deshacerlos.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HistoryAction {
    /// Une en un commit los que van desde `from` (incluido) hasta el último de la rama.
    Squash { from: String, message: String },
    /// Crea un commit que deshace los cambios de `commit`.
    Revert { commit: String },
    /// Mueve la rama a `commit`; con `hard`, también los archivos.
    Reset { commit: String, hard: bool },
    Undo { point: UndoPoint },
}

/// Lo necesario para volver atrás, mientras la rama siga donde la dejó el cambio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UndoPoint {
    pub branch: String,
    pub before: String,
    pub after: String,
    /// Cambios sin confirmar que un `reset --hard` guardó en el stash.
    pub stash: Option<String>,
    /// El cambio movió los archivos: deshacerlo también.
    pub files: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SquashPreview {
    pub branch: String,
    pub commits: usize,
    /// Los mensajes de los commits, del más antiguo al más nuevo.
    pub message: String,
    /// Alguno ya está en una rama remota: publicarlo pedirá un push forzado.
    pub published: bool,
}
