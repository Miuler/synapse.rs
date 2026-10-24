use crate::domain::models::note::Note;
use crate::domain::services::search_service::SearchResult;
pub use crate::infrastructure::navigation::model::{VaultFileDto, VaultFileStatDto};
use crate::infrastructure::navigation::engine::{NavigationEngine, VaultFilesStream};
use serde::{Deserialize, Serialize};
use serde::ser::{SerializeSeq, Serializer};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectVaultFolderResult {
    pub folder_path: String,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntryNode {
    pub name: String,
    pub relative_path: String,
    pub is_folder: bool,
    pub title: Option<String>,
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
    pub total_files: usize,
    pub matched_files: usize,
}
