use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FileVersionState {
    pub index: Option<String>,
    pub worktree: Option<String>,
    pub is_stashed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultVersionState {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub statuses: HashMap<String, FileVersionState>,
    /// Repositorios de la bóveda: el de la raíz (`path` vacío) y los de subcarpetas.
    #[serde(default)]
    pub repos: Vec<RepoInfo>,
}

/// Un repositorio Git dentro de la bóveda.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RepoInfo {
    /// Carpeta del repositorio relativa a la bóveda (`""` si es la propia bóveda o la contiene).
    pub path: String,
    pub branch: Option<String>,
    /// Archivos con cambios (sin confirmar) en ese repositorio.
    pub changes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub is_staged: bool,
    pub old_file_name: String,
    pub new_file_name: String,
    pub old_content: String,
    pub new_content: String,
    pub diff: String,
    /// Diff de trabajo (local contra stage) vacío porque todos los cambios ya están en stage:
    /// se ven con el diff de stage.
    #[serde(default)]
    pub only_staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub ref_name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub last_commit_date: Option<String>,
    pub last_commit_message: Option<String>,
    /// Commits propios que la rama remota (`upstream`) no tiene
    #[serde(default)]
    pub ahead: u32,
    /// Commits de la rama remota que esta rama no tiene todavía
    #[serde(default)]
    pub behind: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchList {
    pub current_branch: Option<String>,
    pub local_branches: Vec<Branch>,
    pub remote_branches: Vec<Branch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileRevision {
    pub path: String,
    pub revision: String,
    pub content: Option<String>,
    pub data_url: Option<String>,
    pub is_binary: bool,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageDiff {
    pub path: String,
    pub is_staged: bool,
    pub diff_data_url: Option<String>,
    pub diff_count: usize,
    pub width: u32,
    pub height: u32,
    pub total_pixels: usize,
    pub diff_percentage: f64,
    pub has_differences: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VersionEntry {
    pub commit_hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub timestamp: i64,
    pub message: String,
    /// Tags que apuntan a este commit (p. ej. `v1.0.0`).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Padres dentro del historial del archivo (commits de esta misma lista o que la lista no
    /// alcanzó a traer): con ellos se dibuja el grafo de ramas.
    #[serde(default)]
    pub parents: Vec<String>,
    /// Qué le pasó al archivo en este commit: `A` (agregado), `M`, `D` (borrado), `R` (renombrado).
    /// `None` en una fusión.
    #[serde(default)]
    pub change: Option<String>,
    /// Con `R`: la ruta que tenía antes.
    #[serde(default)]
    pub previous_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileVersionHistory {
    pub path: String,
    pub commits: Vec<VersionEntry>,
}

/// Referencia que apunta a un commit en el log del repositorio.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommitRef {
    /// Nombre corto (`main`, `origin/main`, `v1.0`).
    pub name: String,
    /// `local`, `remote` o `tag`.
    pub kind: String,
    /// La rama local en la que está HEAD.
    #[serde(default)]
    pub is_head: bool,
}

/// Un commit del log de todas las ramas del repositorio.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RepoCommit {
    pub commit_hash: String,
    pub short_hash: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub date: String,
    pub timestamp: i64,
    pub message: String,
    pub refs: Vec<CommitRef>,
}

/// Log de todas las ramas del repositorio (`git log --all`), del más nuevo al más antiguo.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoLog {
    /// Commit en el que está HEAD.
    pub head: Option<String>,
    pub commits: Vec<RepoCommit>,
}

/// Archivo que cambió en un commit, relativo al repositorio.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CommitFileChange {
    pub path: String,
    /// `A`, `M`, `D`, `R`, `C`, `T`.
    pub change: String,
    pub previous_path: Option<String>,
}

/// Detalles de un commit: mensaje completo y archivos cambiados respecto a su primer padre.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommitDetails {
    pub commit_hash: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub committer_name: String,
    pub date: String,
    pub message: String,
    pub files: Vec<CommitFileChange>,
}
