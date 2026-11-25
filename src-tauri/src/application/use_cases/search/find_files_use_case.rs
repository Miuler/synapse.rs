use crate::application::dto::search::find_files_query::{FindFilesQuery, FoundFile, FoundFiles};
use crate::domain::search::path_pattern::PathPattern;
use crate::domain::vault::vault_item_repository::{VaultFile, VaultItemRepository};

/// Busca archivos de la bóveda por ruta (glob o regex), en cualquier extensión.
pub struct FindFilesUseCase<V: VaultItemRepository> {
    vault_item_repository: V,
}

impl<V: VaultItemRepository> FindFilesUseCase<V> {
    pub fn new(vault_item_repository: V) -> Self {
        Self { vault_item_repository }
    }

    pub fn execute(&self, query: FindFilesQuery) -> Result<FoundFiles, String> {
        let mut patterns = Vec::new();
        if let Some(g) = query.glob.as_deref().filter(|g| !g.trim().is_empty()) {
            patterns.push(PathPattern::glob(g, query.case_sensitive)?);
        }
        if let Some(r) = query.regex.as_deref().filter(|r| !r.is_empty()) {
            patterns.push(PathPattern::regex(r, query.case_sensitive)?);
        }
        let files = self
            .vault_item_repository
            .list_vault_files(&query.vault_path, &query.folder)?;
        let matching: Vec<VaultFile> = files
            .into_iter()
            .filter(|f| has_extension(&f.relative_path, &query.extensions))
            .filter(|f| patterns.iter().all(|p| p.matches(&f.relative_path)))
            .collect();
        Ok(FoundFiles {
            total: matching.len(),
            files: matching
                .into_iter()
                .skip(query.offset)
                .take(query.limit)
                .map(|f| FoundFile { path: f.relative_path, size: f.size })
                .collect(),
        })
    }
}

/// Si la ruta termina en alguna de las extensiones (también compuestas, como `drawio.png`).
pub(crate) fn has_extension(path: &str, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    let lower = path.to_lowercase();
    extensions.iter().any(|e| lower.ends_with(&format!(".{e}")))
}

/// Repositorio en memoria para las pruebas de búsqueda de archivos: (ruta, contenido).
#[cfg(test)]
pub(crate) mod test_support {
    use crate::domain::vault::folder::FolderEntry;
    use crate::domain::vault::vault_item_repository::{VaultFile, VaultItemRepository};
    use std::path::Path;

    pub struct MemoryFiles(pub Vec<(&'static str, &'static [u8])>);

    impl VaultItemRepository for MemoryFiles {
        fn list_vault_files(&self, _: &Path, folder: &str) -> Result<Vec<VaultFile>, String> {
            Ok(self
                .0
                .iter()
                .filter(|(p, _)| folder.is_empty() || p.starts_with(&format!("{folder}/")))
                .map(|(p, c)| VaultFile { relative_path: p.to_string(), size: c.len() as u64 })
                .collect())
        }
        fn read_binary_item(&self, _: &Path, rel: &str) -> Result<Vec<u8>, String> {
            self.0
                .iter()
                .find(|(p, _)| *p == rel)
                .map(|(_, c)| c.to_vec())
                .ok_or_else(|| format!("no existe {rel}"))
        }
        fn exists(&self, _: &Path, rel: &str) -> bool {
            self.0.iter().any(|(p, _)| *p == rel || p.starts_with(&format!("{rel}/")))
        }
        fn delete_item(&self, _: &Path, _: &str) -> Result<(), String> { unimplemented!() }
        fn rename_item(&self, _: &Path, _: &str, _: &str) -> Result<String, String> { unimplemented!() }
        fn move_item(&self, _: &Path, _: &str, _: &str) -> Result<String, String> { unimplemented!() }
        fn copy_item(&self, _: &Path, _: &str, _: &str) -> Result<String, String> { unimplemented!() }
        fn import_item(&self, _: &Path, _: &Path, _: &str) -> Result<String, String> { unimplemented!() }
        fn write_binary_item(&self, _: &Path, _: &str, _: &[u8]) -> Result<(), String> { unimplemented!() }
        fn create_folder(&self, _: &Path, _: &str) -> Result<(), String> { unimplemented!() }
        fn ensure_folder(&self, _: &Path, _: &str) -> Result<(), String> { unimplemented!() }
        fn relocate(&self, _: &Path, _: &str, _: &str) -> Result<(), String> { unimplemented!() }
        fn list_folder(&self, _: &Path, _: &str) -> Result<Vec<FolderEntry>, String> { unimplemented!() }
        fn is_folder(&self, _: &Path, _: &str) -> bool { unimplemented!() }
        fn list_files_recursive(&self, _: &Path, _: &str) -> Result<Vec<String>, String> { unimplemented!() }
        fn list_all_folders(&self, _: &Path) -> Result<Vec<String>, String> { unimplemented!() }
        fn read_text(&self, _: &Path, _: &str) -> Result<String, String> { unimplemented!() }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::MemoryFiles;
    use super::*;
    use std::path::PathBuf;

    fn repo() -> MemoryFiles {
        MemoryFiles(
            [
                "README.md",
                "db/020_schema_ach_operation_ddl.sql",
                "src/infrastructure/entity/OperationMasterEntity.java",
                "src/sequences/BS Financial Gateway.mmd",
                "src/sequences/Flujo.d2",
                "src/soluciones/SBB-SD-000360.archimate",
            ]
            .into_iter()
            .map(|p| (p, &b""[..]))
            .collect(),
        )
    }

    fn query() -> FindFilesQuery {
        FindFilesQuery {
            vault_path: PathBuf::from("/v"),
            glob: None,
            regex: None,
            case_sensitive: false,
            folder: String::new(),
            extensions: vec![],
            limit: 50,
            offset: 0,
        }
    }

    fn paths(r: &FoundFiles) -> Vec<&str> {
        r.files.iter().map(|f| f.path.as_str()).collect()
    }

    #[test]
    fn regex_con_alternativas_como_grep_ie() {
        let r = FindFilesUseCase::new(repo())
            .execute(FindFilesQuery { regex: Some(r"entity|model|mapper|\.sql".into()), ..query() })
            .unwrap();
        assert_eq!(
            paths(&r),
            ["db/020_schema_ach_operation_ddl.sql", "src/infrastructure/entity/OperationMasterEntity.java"]
        );
    }

    #[test]
    fn glob_carpeta_y_extensiones() {
        let r = FindFilesUseCase::new(repo())
            .execute(FindFilesQuery { folder: "src/sequences".into(), ..query() })
            .unwrap();
        assert_eq!(r.total, 2);
        let r = FindFilesUseCase::new(repo())
            .execute(FindFilesQuery { extensions: vec!["archimate".into()], ..query() })
            .unwrap();
        assert_eq!(paths(&r), ["src/soluciones/SBB-SD-000360.archimate"]);
        let r = FindFilesUseCase::new(repo())
            .execute(FindFilesQuery { glob: Some("*ddl*.sql".into()), ..query() })
            .unwrap();
        assert_eq!(paths(&r), ["db/020_schema_ach_operation_ddl.sql"]);
    }

    #[test]
    fn paginacion_con_total_completo() {
        let r = FindFilesUseCase::new(repo())
            .execute(FindFilesQuery { limit: 2, offset: 1, ..query() })
            .unwrap();
        assert_eq!(r.total, 6);
        assert_eq!(r.files.len(), 2);
        assert_eq!(r.files[0].path, "db/020_schema_ach_operation_ddl.sql");
    }
}
