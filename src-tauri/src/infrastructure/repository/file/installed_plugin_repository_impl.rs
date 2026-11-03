use crate::domain::plugin::community_plugin::InstalledPluginInfo;
use crate::domain::plugin::installed_plugin_repository::InstalledPluginRepository;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ManifestExtract {
    id: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    author: Option<String>,
}

/// Plugins instalados en `.synapse/plugins`, leídos de su `manifest.json`.
#[derive(Clone)]
pub struct InstalledPluginRepositoryImpl;

impl InstalledPluginRepository for InstalledPluginRepositoryImpl {
    fn list_installed(&self, vault_path: &Path) -> Result<Vec<InstalledPluginInfo>, String> {
        let mut results = Vec::new();
        let mut seen_ids = HashSet::new();

        let search_dirs = [(
            ".synapse/plugins",
            vault_path.join(".synapse").join("plugins"),
        )];

        for (rel_prefix, abs_dir) in &search_dirs {
            if !abs_dir.exists() || !abs_dir.is_dir() {
                continue;
            }

            let entries = match fs::read_dir(abs_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let entry_path = entry.path();
                if !entry_path.is_dir() {
                    continue;
                }

                let folder_name = entry.file_name().to_string_lossy().to_string();
                let manifest_path = entry_path.join("manifest.json");

                if manifest_path.exists() {
                    if let Ok(content) = fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<ManifestExtract>(&content) {
                            let id = manifest.id.unwrap_or_else(|| folder_name.clone());
                            if seen_ids.contains(&id) {
                                continue;
                            }
                            seen_ids.insert(id.clone());

                            let name = manifest.name.unwrap_or_else(|| id.clone());
                            let version = manifest.version.unwrap_or_else(|| "1.0.0".to_string());
                            let has_styles = entry_path.join("styles.css").exists();
                            let dir = format!("{rel_prefix}/{folder_name}");

                            results.push(InstalledPluginInfo {
                                id,
                                name,
                                version,
                                description: manifest.description,
                                author: manifest.author,
                                dir,
                                has_styles,
                                repo: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scans_installed_plugins() {
        let repo = InstalledPluginRepositoryImpl;
        // Installed plugins scan with temp dir
        let temp = tempdir().unwrap();
        let plugin_dir = temp
            .path()
            .join(".synapse")
            .join("plugins")
            .join("my-cool-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("manifest.json"),
            r#"{"id":"my-cool-plugin","name":"My Cool Plugin","version":"1.2.3"}"#,
        )
        .unwrap();

        let scanned = repo.list_installed(temp.path()).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].id, "my-cool-plugin");
        assert_eq!(scanned[0].name, "My Cool Plugin");
        assert_eq!(scanned[0].version, "1.2.3");
    }
}
