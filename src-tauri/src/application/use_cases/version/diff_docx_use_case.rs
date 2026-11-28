use crate::application::dto::version::docx_source_query::{DocxSource, DocxSourceQuery};
use crate::domain::docx::{DocxDiff, DocxDiffer, DocxDocument, DocxParser};
use crate::domain::vault::vault_item_repository::VaultItemRepository;
use crate::domain::version::version_control::VersionControl;
use std::path::Path;

/// Compara dos versiones de un documento de Word: disco, stage, una revisión u otro archivo.
#[derive(Clone)]
pub struct DiffDocxUseCase<V, I, P, D> {
    version_control: V,
    vault_items: I,
    parser: P,
    differ: D,
}

impl<V: VersionControl, I: VaultItemRepository, P: DocxParser, D: DocxDiffer> DiffDocxUseCase<V, I, P, D> {
    pub fn new(version_control: V, vault_items: I, parser: P, differ: D) -> Self {
        Self { version_control, vault_items, parser, differ }
    }

    pub fn execute(&self, vault_path: &Path, before: &DocxSourceQuery, after: &DocxSourceQuery) -> Result<DocxDiff, String> {
        let old = self.read(vault_path, before)?;
        let new = self.read(vault_path, after)?;
        if old.is_none() && new.is_none() {
            return Err(format!("{} does not exist in either version", after.path));
        }
        let mut diff = DocxDiff {
            old_exists: old.is_some(),
            new_exists: new.is_some(),
            same_bytes: old == new,
            ..DocxDiff::default()
        };
        // Un documento que no existía cuenta como vacío
        let parse = |bytes: &Option<Vec<u8>>| match bytes {
            Some(b) => self.parser.parse(b),
            None => Ok(DocxDocument::default()),
        };
        (diff.entries, diff.approximate) = self.differ.diff(&parse(&old)?, &parse(&new)?);
        Ok(diff)
    }

    /// Bytes de una versión; `None` si no existe.
    pub fn read(&self, vault_path: &Path, query: &DocxSourceQuery) -> Result<Option<Vec<u8>>, String> {
        match &query.source {
            DocxSource::Disk if !self.vault_items.exists(vault_path, &query.path) => Ok(None),
            DocxSource::Disk => self.vault_items.read_binary_item(vault_path, &query.path).map(Some),
            DocxSource::Staged => self.version_control.get_file_bytes(vault_path, &query.path, "staged"),
            DocxSource::Revision(rev) => self.version_control.get_file_bytes(vault_path, &query.path, rev),
        }
    }

    /// Una imagen de una versión, por su huella.
    pub fn image(&self, vault_path: &Path, query: &DocxSourceQuery, sha256: &str) -> Result<Option<(String, Vec<u8>)>, String> {
        match self.read(vault_path, query)? {
            Some(bytes) => self.parser.image(&bytes, sha256),
            None => Ok(None),
        }
    }
}
