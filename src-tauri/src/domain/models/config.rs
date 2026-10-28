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
pub struct WelcomeShortcutsConfig {
    #[serde(default = "default_welcome_new_workspace")]
    pub new_workspace: String,
    #[serde(default = "default_welcome_open_folder")]
    pub open_folder: String,
    #[serde(default = "default_welcome_clone_git")]
    pub clone_git: String,
    #[serde(default = "default_welcome_search_workspaces")]
    pub search_workspaces: String,
}

fn default_welcome_new_workspace() -> String {
    "Ctrl+Shift+N".to_string()
}
fn default_welcome_open_folder() -> String {
    "Ctrl+Shift+O".to_string()
}
fn default_welcome_clone_git() -> String {
    "Ctrl+Shift+G".to_string()
}
fn default_welcome_search_workspaces() -> String {
    "Ctrl+F".to_string()
}

impl Default for WelcomeShortcutsConfig {
    fn default() -> Self {
        Self {
            new_workspace: default_welcome_new_workspace(),
            open_folder: default_welcome_open_folder(),
            clone_git: default_welcome_clone_git(),
            search_workspaces: default_welcome_search_workspaces(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceShortcutsConfig {
    #[serde(default = "default_ws_quick_open")]
    pub quick_open: String,
    #[serde(default = "default_ws_command_palette")]
    pub command_palette: String,
    #[serde(default = "default_ws_new_file")]
    pub new_file: String,
    #[serde(default = "default_ws_new_tab")]
    pub new_tab: String,
    #[serde(default = "default_ws_save_file")]
    pub save_file: String,
    #[serde(default = "default_ws_close_tab")]
    pub close_tab: String,
    #[serde(default = "default_ws_split_view")]
    pub split_view: String,
    #[serde(default = "default_ws_toggle_sidebar")]
    pub toggle_sidebar: String,
    #[serde(default = "default_ws_locate_active_file")]
    pub locate_active_file: String,
    #[serde(default = "default_ws_search_fulltext")]
    pub search_fulltext: String,
    #[serde(default = "default_ws_open_vault_folder")]
    pub open_vault_folder: String,
    #[serde(default = "default_ws_reload_active_file")]
    pub reload_active_file: String,
    #[serde(default = "default_ws_reload_vault")]
    pub reload_vault: String,
    #[serde(default = "default_ws_nav_back")]
    pub nav_back: String,
    #[serde(default = "default_ws_nav_forward")]
    pub nav_forward: String,
    #[serde(default = "default_ws_toggle_theme")]
    pub toggle_theme: String,
    #[serde(default = "default_ws_toggle_devtools")]
    pub toggle_devtools: String,
    #[serde(default = "default_ws_toggle_edit_mode")]
    pub toggle_edit_mode: String,
    #[serde(default = "default_ws_next_tab")]
    pub next_tab: String,
    #[serde(default = "default_ws_prev_tab")]
    pub prev_tab: String,
    #[serde(default = "default_ws_git_push")]
    pub git_push: String,
}

fn default_ws_quick_open() -> String {
    "Ctrl+O".to_string()
}
fn default_ws_command_palette() -> String {
    "Ctrl+P".to_string()
}
fn default_ws_new_file() -> String {
    "Ctrl+N".to_string()
}
fn default_ws_new_tab() -> String {
    "Ctrl+T".to_string()
}
fn default_ws_save_file() -> String {
    "Ctrl+S".to_string()
}
fn default_ws_close_tab() -> String {
    "Ctrl+W, Ctrl+F4".to_string()
}
fn default_ws_split_view() -> String {
    "Ctrl+\\".to_string()
}
fn default_ws_toggle_sidebar() -> String {
    "Alt+1, Ctrl+B".to_string()
}
fn default_ws_locate_active_file() -> String {
    "Ctrl+Alt+1, Alt+L".to_string()
}
fn default_ws_search_fulltext() -> String {
    "Ctrl+Shift+F".to_string()
}
fn default_ws_open_vault_folder() -> String {
    "Ctrl+Shift+O".to_string()
}
fn default_ws_reload_active_file() -> String {
    "Ctrl+R".to_string()
}
fn default_ws_reload_vault() -> String {
    "Ctrl+Shift+R".to_string()
}
fn default_ws_nav_back() -> String {
    "Alt+ArrowLeft".to_string()
}
fn default_ws_nav_forward() -> String {
    "Alt+ArrowRight".to_string()
}
fn default_ws_toggle_theme() -> String {
    "Ctrl+Shift+T".to_string()
}
fn default_ws_toggle_devtools() -> String {
    "F12, Ctrl+Shift+I".to_string()
}
fn default_ws_toggle_edit_mode() -> String {
    "Ctrl+E".to_string()
}
fn default_ws_next_tab() -> String {
    "Ctrl+PageDown, Ctrl+Tab".to_string()
}
fn default_ws_prev_tab() -> String {
    "Ctrl+PageUp, Ctrl+Shift+Tab".to_string()
}
fn default_ws_git_push() -> String {
    "Ctrl+Shift+K".to_string()
}

impl Default for WorkspaceShortcutsConfig {
    fn default() -> Self {
        Self {
            quick_open: default_ws_quick_open(),
            command_palette: default_ws_command_palette(),
            new_file: default_ws_new_file(),
            new_tab: default_ws_new_tab(),
            save_file: default_ws_save_file(),
            close_tab: default_ws_close_tab(),
            split_view: default_ws_split_view(),
            toggle_sidebar: default_ws_toggle_sidebar(),
            locate_active_file: default_ws_locate_active_file(),
            search_fulltext: default_ws_search_fulltext(),
            open_vault_folder: default_ws_open_vault_folder(),
            reload_active_file: default_ws_reload_active_file(),
            reload_vault: default_ws_reload_vault(),
            nav_back: default_ws_nav_back(),
            nav_forward: default_ws_nav_forward(),
            toggle_theme: default_ws_toggle_theme(),
            toggle_devtools: default_ws_toggle_devtools(),
            toggle_edit_mode: default_ws_toggle_edit_mode(),
            next_tab: default_ws_next_tab(),
            prev_tab: default_ws_prev_tab(),
            git_push: default_ws_git_push(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ShortcutsConfig {
    #[serde(default)]
    pub welcome: WelcomeShortcutsConfig,
    #[serde(default)]
    pub workspace: WorkspaceShortcutsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorConfig {
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,
    #[serde(default = "default_auto_save_delay_ms")]
    pub auto_save_delay_ms: u64,
}

fn default_auto_save() -> bool {
    true
}

fn default_auto_save_delay_ms() -> u64 {
    600
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            auto_save: default_auto_save(),
            auto_save_delay_ms: default_auto_save_delay_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexingConfig {
    #[serde(default = "default_max_pdf_file_size")]
    pub max_pdf_file_size: usize,
    #[serde(default = "default_max_pdf_pages")]
    pub max_pdf_pages: usize,
    #[serde(default = "default_max_docx_file_size")]
    pub max_docx_file_size: usize,
    #[serde(default = "default_max_extracted_chars")]
    pub max_extracted_chars: usize,
    #[serde(default = "default_max_word_length")]
    pub max_word_length: usize,
}

fn default_max_pdf_file_size() -> usize {
    31_457_280 // 30 MB
}

fn default_max_pdf_pages() -> usize {
    80
}

fn default_max_docx_file_size() -> usize {
    15_728_640 // 15 MB
}

fn default_max_extracted_chars() -> usize {
    200_000
}

fn default_max_word_length() -> usize {
    80
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            max_pdf_file_size: default_max_pdf_file_size(),
            max_pdf_pages: default_max_pdf_pages(),
            max_docx_file_size: default_max_docx_file_size(),
            max_extracted_chars: default_max_extracted_chars(),
            max_word_length: default_max_word_length(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffConfig {
    #[serde(default = "default_diff_max_expand_lines")]
    pub max_expand_lines: usize,
}

fn default_diff_max_expand_lines() -> usize {
    800
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            max_expand_lines: default_diff_max_expand_lines(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(alias = "supported_files.jsonc")]
    pub supported_files: SupportedFileTypes,
    #[serde(default)]
    pub ignored: IgnoredConfig,
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub indexing: IndexingConfig,
    #[serde(default)]
    pub diff: DiffConfig,
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

        assert_eq!(config.shortcuts.welcome.new_workspace, "Ctrl+Shift+N");
        assert_eq!(config.shortcuts.welcome.open_folder, "Ctrl+Shift+O");
        assert_eq!(config.shortcuts.workspace.quick_open, "Ctrl+O");
        assert_eq!(config.shortcuts.workspace.command_palette, "Ctrl+P");
        assert_eq!(config.shortcuts.workspace.toggle_edit_mode, "Ctrl+E");

        assert_eq!(config.indexing.max_pdf_file_size, 31_457_280);
        assert_eq!(config.indexing.max_pdf_pages, 80);
        assert_eq!(config.indexing.max_docx_file_size, 15_728_640);
        assert_eq!(config.indexing.max_extracted_chars, 200_000);
        assert_eq!(config.indexing.max_word_length, 80);

        assert_eq!(config.diff.max_expand_lines, 800);
    }
}
