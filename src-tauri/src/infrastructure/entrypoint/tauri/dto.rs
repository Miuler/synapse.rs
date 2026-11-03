use crate::domain::note::note_path::NoteRelativePath;
use crate::domain::search::note_search_repository::SearchResult;
use crate::infrastructure::navigation::engine::{NavigationEngine, VaultFilesStream};
pub use crate::infrastructure::navigation::model::{VaultFileDto, VaultFileStatDto};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectVaultFolderResult {
    pub folder_path: String,
    pub notes: Vec<VaultEntryDto>,
}

/// Elemento de la lista de la bóveda que recibe el front: los datos del explorador y los
/// metadatos de archivo. La fachada de Obsidian construye su índice con esta misma lista, sin
/// pedir otra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntryDto {
    pub relative_path: String,
    pub abs_path: String,
    pub title: String,
    pub view_mode: Option<String>,
    /// Milisegundos desde la época.
    pub ctime: Option<u128>,
    pub mtime: u128,
    pub size: u64,
}

impl VaultEntryDto {
    /// Lista ordenada por ruta a partir del motor de navegación (sin leer el disco).
    pub fn list(vault_path: &Path, engine: &NavigationEngine, with_view_mode: bool) -> Vec<Self> {
        let mut entries = Vec::with_capacity(engine.notes.len());
        for item in engine.notes.iter() {
            let meta = item.value();
            let rel_str = meta.path.to_string();
            // Mismo filtro que antes: solo rutas relativas válidas
            if NoteRelativePath::new(&rel_str).is_err() {
                continue;
            }
            entries.push(Self {
                abs_path: vault_path.join(&rel_str).to_string_lossy().to_string(),
                relative_path: rel_str,
                title: meta.title.to_string(),
                view_mode: if with_view_mode {
                    meta.view_mode.as_ref().map(|s| s.to_string())
                } else {
                    None
                },
                ctime: meta.created_nanos.map(|value| value / 1_000_000),
                mtime: meta.mtime_nanos / 1_000_000,
                size: meta.size_bytes,
            });
        }
        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        entries
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntryNode {
    pub name: String,
    pub relative_path: String,
    pub is_folder: bool,
    pub title: Option<String>,
    #[serde(default)]
    pub sync_state: Option<String>,
}

pub struct VaultFilesResponse {
    pub engine: Option<Arc<NavigationEngine>>,
}

impl Serialize for VaultFilesResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(engine) = &self.engine {
            VaultFilesStream {
                engine: Arc::clone(engine),
            }
            .serialize(serializer)
        } else {
            let seq = serializer.serialize_seq(Some(0))?;
            seq.end()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuickOpenSearchResult {
    pub results: Vec<SearchResult>,
    #[serde(default)]
    pub folders: Vec<crate::domain::vault::folder::FolderMeta>,
    pub total_files: usize,
    pub matched_files: usize,
}
