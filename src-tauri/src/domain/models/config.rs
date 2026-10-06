use super::file_types::SupportedFileTypes;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IgnoredConfig {
    #[serde(default)]
    pub directories: Vec<String>,
    #[serde(default)]
    pub suffixes: Vec<String>,
    #[serde(default)]
    pub prefixes: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

impl IgnoredConfig {
    pub fn is_ignored_dir_or_file(&self, name: &str) -> bool {
        if name.is_empty() || name == "." || name == ".." {
            return false;
        }
        name.starts_with('.')
            || self.directories.iter().any(|d| d.eq_ignore_ascii_case(name))
            || self.files.iter().any(|f| f.eq_ignore_ascii_case(name))
    }

    pub fn should_ignore_path(&self, rel_path: &Path) -> bool {
        let rel_str = rel_path.as_os_str().to_string_lossy();
        if rel_str.is_empty() || rel_str == "." {
            return true;
        }

        // Ignore any path containing hidden components or ignored folders
        for component in rel_path.components() {
            if let std::path::Component::Normal(comp) = component {
                let comp_str = comp.to_string_lossy();
                if self.is_ignored_dir_or_file(&comp_str) {
                    return true;
                }
            }
        }

        if let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) {
            let lower = file_name.to_ascii_lowercase();
            // Check prefixes (e.g. #, .#)
            if self.prefixes.iter().any(|p| file_name.starts_with(p)) {
                return true;
            }
            // Check suffixes (e.g. ~, .tmp, .temp, .swp, .lock, etc.)
            if self.suffixes.iter().any(|s| {
                if s.starts_with('.') {
                    lower.ends_with(&s.to_ascii_lowercase())
                } else {
                    file_name.ends_with(s.as_str())
                }
            }) {
                return true;
            }
            // Check exact file names
            if self.files.iter().any(|f| f.eq_ignore_ascii_case(file_name)) {
                return true;
            }
        }

        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(alias = "supported_files.jsonc")]
    pub supported_files: SupportedFileTypes,
    #[serde(default)]
    pub ignored: IgnoredConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        const RAW_JSONC: &str = include_str!("../../../../config.jsonc");
        serde_json_lenient::from_str(RAW_JSONC).expect("Error al deserializar config.jsonc")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_load_and_ignore_rules() {
        let config = AppConfig::default();
        assert!(config.supported_files.markdown.contains(&"md".to_string()));
        assert!(config.ignored.directories.contains(&"node_modules".to_string()));
        assert!(config.ignored.suffixes.contains(&".tmp".to_string()));

        assert!(config.ignored.is_ignored_dir_or_file("node_modules"));
        assert!(config.ignored.is_ignored_dir_or_file("target"));
        assert!(config.ignored.is_ignored_dir_or_file(".git"));
        assert!(!config.ignored.is_ignored_dir_or_file("mis_notas"));

        assert!(config.ignored.should_ignore_path(Path::new("")));
        assert!(config.ignored.should_ignore_path(Path::new(".")));
        assert!(config.ignored.should_ignore_path(Path::new("node_modules/pkg/index.js")));
        assert!(config.ignored.should_ignore_path(Path::new(".git/config")));
        assert!(config.ignored.should_ignore_path(Path::new("docs/.git/config")));
        assert!(config.ignored.should_ignore_path(Path::new(".obsidian/workspace.json")));
        assert!(config.ignored.should_ignore_path(Path::new(".cache/foo")));
        assert!(config.ignored.should_ignore_path(Path::new(".anything_hidden/note.md")));
        assert!(config.ignored.should_ignore_path(Path::new("sub/.hidden/note.md")));
        assert!(config.ignored.should_ignore_path(Path::new("draft.tmp")));
        assert!(config.ignored.should_ignore_path(Path::new("backup.md~")));
        assert!(config.ignored.should_ignore_path(Path::new("#autosave.md#")));
        assert!(config.ignored.should_ignore_path(Path::new("sync.lock")));
        assert!(!config.ignored.should_ignore_path(Path::new("docs/intro.md")));
        assert!(!config.ignored.should_ignore_path(Path::new("./docs/intro.md")));
    }
}
