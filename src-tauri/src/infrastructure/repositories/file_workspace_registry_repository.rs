use crate::domain::models::config::{AppConfig, ShortcutsConfig};
use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::workspace_registry::{RecentWorkspaceEntry, WorkspaceRegistryConfig};
use crate::domain::repositories::workspace_registry_repository::WorkspaceRegistryRepository;
use crate::infrastructure::navigation::watcher::{is_ignored_dir_or_file, should_ignore_path};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FileWorkspaceRegistryRepository {
    config_file_path: PathBuf,
    file_types: SupportedFileTypes,
}

impl FileWorkspaceRegistryRepository {
    pub fn new(file_types: SupportedFileTypes) -> Self {
        let config_dir = Self::resolve_config_dir();
        Self {
            config_file_path: config_dir.join("workspaces.json"),
            file_types,
        }
    }

    pub fn with_config_dir(config_dir: PathBuf, file_types: SupportedFileTypes) -> Self {
        Self {
            config_file_path: config_dir.join("workspaces.json"),
            file_types,
        }
    }

    pub fn resolve_config_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            let trimmed = home.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join(".config").join("synapsers");
            }
        }
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let trimmed = xdg.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("synapsers");
            }
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let trimmed = userprofile.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join(".config").join("synapsers");
            }
        }
        PathBuf::from(".config").join("synapsers")
    }

    pub fn format_display_path(path: &Path) -> String {
        let raw = path.to_string_lossy().to_string();
        if let Ok(home) = std::env::var("HOME") {
            let clean_home = home.trim_end_matches('/');
            if !clean_home.is_empty() {
                if raw == clean_home {
                    return "~".to_string();
                }
                let prefix = format!("{}/", clean_home);
                if let Some(rest) = raw.strip_prefix(&prefix) {
                    return format!("~/{}", rest);
                }
            }
        }
        raw
    }

    pub fn extract_workspace_name(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("Workspace")
            .to_string()
    }

    pub fn detect_git_metadata(path: &Path) -> (bool, Option<String>) {
        if !path.is_dir() {
            return (false, None);
        }
        match gix::discover(path) {
            Ok(repo) => {
                let branch = repo
                    .head_name()
                    .ok()
                    .flatten()
                    .map(|n| n.shorten().to_string());
                (true, branch)
            }
            Err(_) => (false, None),
        }
    }

    pub fn count_supported_notes(&self, path: &Path) -> usize {
        if !path.is_dir() {
            return 0;
        }
        let file_types = &self.file_types;
        let mut count = 0usize;
        for entry_res in jwalk::WalkDir::new(path)
            .skip_hidden(true)
            .process_read_dir(|depth, _dir, _state, children| {
                if depth.is_none() {
                    return;
                }
                children.retain(|res| {
                    res.as_ref()
                        .map(|e| {
                            let name = e.file_name.to_string_lossy();
                            !is_ignored_dir_or_file(&name)
                        })
                        .unwrap_or(false)
                });
            })
        {
            if let Ok(entry) = entry_res {
                if entry.file_type.is_file() {
                    let abs = entry.path();
                    if let Ok(rel) = abs.strip_prefix(path) {
                        if !should_ignore_path(rel) {
                            let file_name = entry.file_name.to_string_lossy();
                            if file_types.is_supported_file(&file_name) {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        count
    }

    fn ensure_dir_exists(&self) -> Result<(), String> {
        if let Some(parent) = self.config_file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "No se pudo crear el directorio de configuración {:?}: {}",
                    parent, e
                )
            })?;
        }
        Ok(())
    }

    fn normalize_path(path_str: &str) -> String {
        let p = PathBuf::from(path_str.trim());
        if let Ok(canon) = std::fs::canonicalize(&p) {
            canon.to_string_lossy().to_string()
        } else {
            p.to_string_lossy().trim_end_matches('/').to_string()
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn default_shortcuts_jsonc_template() -> &'static str {
        r#"{
  // ============================================================================
  // Configuración de Atajos de Teclado (Shortcuts) de Synapse.rs
  // Formato: JSONC (soporta comentarios // y /* */)
  // Puedes personalizar cualquier combinación de teclas aquí.
  // ============================================================================

  // Atajos de la Pantalla de Bienvenida (Workspaces Recientes)
  "welcome": {
    // Crear un nuevo directorio de Workspace
    "new_workspace": "Ctrl+Shift+N",
    // Abrir carpeta local existente como Workspace
    "open_folder": "Ctrl+Shift+O",
    // Clonar un repositorio Git como Workspace
    "clone_git": "Ctrl+Shift+G",
    // Enfocar el buscador de Workspaces recientes
    "search_workspaces": "Ctrl+F"
  },

  // Atajos del Espacio de Trabajo y Editor
  "workspace": {
    // Buscar y abrir archivo rápidamente por nombre (Quick Open)
    "quick_open": "Ctrl+O",
    // Abrir la paleta de comandos
    "command_palette": "Ctrl+P",
    // Crear nueva nota Markdown en la bóveda activa
    "new_file": "Ctrl+N",
    // Abrir una nueva pestaña vacía
    "new_tab": "Ctrl+T",
    // Guardar / Grabar el archivo activo
    "save_file": "Ctrl+S",
    // Cerrar la pestaña activa
    "close_tab": "Ctrl+W, Ctrl+F4",
    // Dividir el panel verticalmente (Split View)
    "split_view": "Ctrl+\\",
    // Mostrar u ocultar el explorador lateral de archivos
    "toggle_sidebar": "Alt+1, Ctrl+B",
    // Ubicar el archivo activo en el árbol del explorador
    "locate_active_file": "Ctrl+Alt+1, Alt+L",
    // Buscar en el contenido de las notas (Full-Text Search)
    "search_fulltext": "Ctrl+Shift+F",
    // Abrir otra carpeta o bóveda
    "open_vault_folder": "Ctrl+Shift+O",
    // Recargar el archivo activo desde el disco
    "reload_active_file": "Ctrl+R",
    // Recargar toda la bóveda y sus metadatos
    "reload_vault": "Ctrl+Shift+R",
    // Navegar atrás en el historial de pestañas
    "nav_back": "Alt+ArrowLeft",
    // Navegar adelante en el historial de pestañas
    "nav_forward": "Alt+ArrowRight",
    // Alternar entre tema Claro y Oscuro
    "toggle_theme": "Ctrl+Shift+T",
    // Abrir o cerrar herramientas de desarrollo (DevTools)
    "toggle_devtools": "F12, Ctrl+Shift+I"
  }
}
"#
    }

    fn shortcuts_file_path(&self) -> PathBuf {
        self.config_file_path
            .parent()
            .map(|p| p.join("shortcuts.jsonc"))
            .unwrap_or_else(|| PathBuf::from("shortcuts.jsonc"))
    }
}

impl WorkspaceRegistryRepository for FileWorkspaceRegistryRepository {
    fn load_registry(&self) -> Result<WorkspaceRegistryConfig, String> {
        self.ensure_dir_exists()?;
        if !self.config_file_path.exists() {
            let default_cfg = WorkspaceRegistryConfig::default();
            self.save_registry(&default_cfg)?;
            return Ok(default_cfg);
        }

        let raw = std::fs::read_to_string(&self.config_file_path).map_err(|e| {
            format!(
                "Error leyendo registro de workspaces {:?}: {}",
                self.config_file_path, e
            )
        })?;

        if raw.trim().is_empty() {
            return Ok(WorkspaceRegistryConfig::default());
        }

        serde_json::from_str::<WorkspaceRegistryConfig>(&raw).map_err(|e| {
            format!(
                "Error deserializando registro de workspaces {:?}: {}",
                self.config_file_path, e
            )
        })
    }

    fn save_registry(&self, config: &WorkspaceRegistryConfig) -> Result<(), String> {
        self.ensure_dir_exists()?;
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Error serializando registro de workspaces: {}", e))?;
        std::fs::write(&self.config_file_path, json).map_err(|e| {
            format!(
                "Error guardando registro de workspaces en {:?}: {}",
                self.config_file_path, e
            )
        })
    }

    fn record_workspace_opened(
        &self,
        path: &str,
        notes_count: Option<usize>,
        git_branch: Option<String>,
        is_git_repo: Option<bool>,
    ) -> Result<RecentWorkspaceEntry, String> {
        let clean_path = Self::normalize_path(path);
        let path_buf = PathBuf::from(&clean_path);
        let exists = path_buf.is_dir();

        let mut cfg = self.load_registry().unwrap_or_default();

        let existing_pinned = cfg
            .workspaces
            .iter()
            .find(|w| w.path == clean_path)
            .map(|w| w.pinned)
            .unwrap_or(false);

        let existing_notes = cfg
            .workspaces
            .iter()
            .find(|w| w.path == clean_path)
            .map(|w| w.notes_count);

        let computed_notes = notes_count
            .or(existing_notes)
            .unwrap_or_else(|| self.count_supported_notes(&path_buf));

        let (detected_is_git, detected_branch) = if is_git_repo.is_some() || git_branch.is_some() {
            (is_git_repo.unwrap_or(git_branch.is_some()), git_branch)
        } else {
            Self::detect_git_metadata(&path_buf)
        };

        let entry = RecentWorkspaceEntry {
            path: clean_path.clone(),
            display_path: Self::format_display_path(&path_buf),
            name: Self::extract_workspace_name(&path_buf),
            last_opened_ms: Self::now_ms(),
            pinned: existing_pinned,
            notes_count: computed_notes,
            git_branch: detected_branch,
            is_git_repo: detected_is_git,
            exists,
        };

        cfg.workspaces.retain(|w| w.path != clean_path);
        cfg.workspaces.insert(0, entry.clone());
        cfg.last_active_workspace = Some(clean_path);

        self.save_registry(&cfg)?;
        Ok(entry)
    }

    fn toggle_pin(&self, path: &str) -> Result<Vec<RecentWorkspaceEntry>, String> {
        let clean_path = Self::normalize_path(path);
        let mut cfg = self.load_registry().unwrap_or_default();

        for w in cfg.workspaces.iter_mut() {
            if w.path == clean_path || w.path == path {
                w.pinned = !w.pinned;
                break;
            }
        }

        self.save_registry(&cfg)?;
        self.list_workspaces()
    }

    fn remove_workspace(&self, path: &str) -> Result<Vec<RecentWorkspaceEntry>, String> {
        let clean_path = Self::normalize_path(path);
        let mut cfg = self.load_registry().unwrap_or_default();

        cfg.workspaces
            .retain(|w| w.path != clean_path && w.path != path);
        if cfg.last_active_workspace.as_deref() == Some(&clean_path)
            || cfg.last_active_workspace.as_deref() == Some(path)
        {
            cfg.last_active_workspace = cfg.workspaces.first().map(|w| w.path.clone());
        }

        self.save_registry(&cfg)?;
        self.list_workspaces()
    }

    fn list_workspaces(&self) -> Result<Vec<RecentWorkspaceEntry>, String> {
        let mut cfg = self.load_registry().unwrap_or_default();
        let mut changed = false;

        for w in cfg.workspaces.iter_mut() {
            let p = PathBuf::from(&w.path);
            let exists = p.is_dir();
            if w.exists != exists {
                w.exists = exists;
                changed = true;
            }
            let display = Self::format_display_path(&p);
            if w.display_path != display {
                w.display_path = display;
                changed = true;
            }
            if exists {
                let (is_git, branch) = Self::detect_git_metadata(&p);
                if is_git {
                    if !w.is_git_repo || (branch.is_some() && w.git_branch != branch) {
                        w.is_git_repo = true;
                        w.git_branch = branch;
                        changed = true;
                    }
                }
                if w.notes_count == 0 {
                    let c = self.count_supported_notes(&p);
                    if c > 0 {
                        w.notes_count = c;
                        changed = true;
                    }
                }
            }
        }

        if changed {
            let _ = self.save_registry(&cfg);
        }

        Ok(cfg.workspaces)
    }

    fn load_shortcuts_config(&self) -> Result<ShortcutsConfig, String> {
        self.ensure_dir_exists()?;
        let shortcuts_path = self.shortcuts_file_path();

        if !shortcuts_path.exists() {
            let _ = std::fs::write(&shortcuts_path, Self::default_shortcuts_jsonc_template());
        }

        if let Ok(raw) = std::fs::read_to_string(&shortcuts_path) {
            if !raw.trim().is_empty() {
                if let Ok(parsed) = serde_json_lenient::from_str::<ShortcutsConfig>(&raw) {
                    return Ok(parsed);
                }
            }
        }

        Ok(AppConfig::default().shortcuts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_workspace_registry_crud_and_persistence() {
        let cfg_dir = tempdir().unwrap();
        let ws1_dir = tempdir().unwrap();
        let ws2_dir = tempdir().unwrap();

        // Create some markdown notes in ws1
        let note1 = ws1_dir.path().join("intro.md");
        let mut f1 = File::create(&note1).unwrap();
        writeln!(f1, "# Intro").unwrap();

        let repo = FileWorkspaceRegistryRepository::with_config_dir(
            cfg_dir.path().to_path_buf(),
            SupportedFileTypes::default(),
        );

        // Initial load creates workspaces.json
        let initial = repo.load_registry().unwrap();
        assert!(initial.workspaces.is_empty());
        assert!(cfg_dir.path().join("workspaces.json").exists());

        // Record first workspace
        let ws1_str = ws1_dir.path().to_string_lossy().to_string();
        let entry1 = repo
            .record_workspace_opened(&ws1_str, None, None, None)
            .unwrap();
        assert_eq!(entry1.notes_count, 1);
        assert!(entry1.exists);
        assert!(!entry1.pinned);

        // Record second workspace
        let ws2_str = ws2_dir.path().to_string_lossy().to_string();
        let _entry2 = repo
            .record_workspace_opened(&ws2_str, Some(42), Some("main".to_string()), Some(true))
            .unwrap();

        let list = repo.list_workspaces().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].notes_count, 42);
        assert_eq!(list[0].git_branch, Some("main".to_string()));
        assert!(list[0].is_git_repo);

        // Toggle pin on ws1
        let after_pin = repo.toggle_pin(&ws1_str).unwrap();
        let ws1_after = after_pin.iter().find(|w| w.notes_count == 1).unwrap();
        assert!(ws1_after.pinned);

        // Remove ws2
        let after_remove = repo.remove_workspace(&ws2_str).unwrap();
        assert_eq!(after_remove.len(), 1);
        assert_eq!(after_remove[0].notes_count, 1);

        // Verify shortcuts.jsonc creation and JSONC comment support
        let shortcuts = repo.load_shortcuts_config().unwrap();
        assert!(cfg_dir.path().join("shortcuts.jsonc").exists());
        assert_eq!(shortcuts.welcome.new_workspace, "Ctrl+Shift+N");
        assert_eq!(shortcuts.workspace.quick_open, "Ctrl+O");

        // Customize shortcuts.jsonc with comments and verify reload
        let custom_jsonc = r#"{
            // Comentario personalizado
            "welcome": {
                "new_workspace": "Ctrl+Alt+N"
            },
            "workspace": {
                /* Comentario en bloque */
                "quick_open": "Ctrl+K"
            }
        }"#;
        std::fs::write(cfg_dir.path().join("shortcuts.jsonc"), custom_jsonc).unwrap();
        let reloaded = repo.load_shortcuts_config().unwrap();
        assert_eq!(reloaded.welcome.new_workspace, "Ctrl+Alt+N");
        assert_eq!(reloaded.welcome.open_folder, "Ctrl+Shift+O"); // Default preserved
        assert_eq!(reloaded.workspace.quick_open, "Ctrl+K");
    }
}
