use crate::domain::appearance::{GlobalTheme, GlobalThemeRepository};
use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Lee las carpetas de temas globales de `<config>/config.jsonc` (`appearance.theme_dirs`) y los
/// `.css` que hay en ellas. `<config>/themes` se lee siempre y no forma parte de esa lista.
#[derive(Clone)]
pub struct GlobalThemeRepositoryImpl {
    config_dir: PathBuf,
    home: Option<PathBuf>,
}

const HEADER: &str = "// Configuración global de Synapse: vale para todas las bóvedas.\n\
// appearance.theme_dirs: carpetas extra con temas CSS propios (*.css). «~» es la carpeta del usuario.\n\
// ~/.config/synapsers/themes se lee siempre, sin necesidad de ponerla aquí.\n\
// Se editan también desde Configuración → Apariencia.\n";

impl GlobalThemeRepositoryImpl {
    pub fn new() -> Self {
        Self::with_dirs(
            WorkspaceRegistryRepositoryImpl::resolve_config_dir(),
            std::env::var_os("HOME").map(PathBuf::from),
        )
    }

    pub fn with_dirs(config_dir: PathBuf, home: Option<PathBuf>) -> Self {
        Self { config_dir, home }
    }

    fn config_path(&self) -> PathBuf {
        self.config_dir.join("config.jsonc")
    }

    /// `<config>/themes`, con `~` si está dentro de la carpeta del usuario.
    fn default_dir_display(&self) -> String {
        let dir = self.config_dir.join("themes");
        match self.home.as_deref().and_then(|h| dir.strip_prefix(h).ok()) {
            Some(rest) => format!("~/{}", rest.to_string_lossy()),
            None => dir.to_string_lossy().into_owned(),
        }
    }

    fn expand(&self, dir: &str) -> PathBuf {
        match (
            dir.strip_prefix("~/")
                .or(if dir == "~" { Some("") } else { None }),
            &self.home,
        ) {
            (Some(rest), Some(home)) => home.join(rest),
            _ => PathBuf::from(dir),
        }
    }

    fn read_config(&self) -> Value {
        std::fs::read_to_string(self.config_path())
            .ok()
            .and_then(|raw| serde_json_lenient::from_str::<Value>(&raw).ok())
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}))
    }

    fn write_config(&self, config: &Value) -> Result<(), String> {
        std::fs::create_dir_all(&self.config_dir)
            .map_err(|e| format!("No se pudo crear {}: {e}", self.config_dir.display()))?;
        let body = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
        std::fs::write(self.config_path(), format!("{HEADER}{body}\n"))
            .map_err(|e| format!("No se pudo guardar {}: {e}", self.config_path().display()))
    }

    fn css_files(dir: &Path) -> Vec<(String, PathBuf)> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return vec![];
        };
        let mut files: Vec<(String, PathBuf)> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter_map(|p| {
                let name = p.file_name()?.to_str()?;
                let id = name
                    .strip_suffix(".css")
                    .or_else(|| name.strip_suffix(".CSS"))?;
                Some((id.to_string(), p.clone()))
            })
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    }
}

impl Default for GlobalThemeRepositoryImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalThemeRepository for GlobalThemeRepositoryImpl {
    fn default_theme_dir(&self) -> String {
        self.default_dir_display()
    }

    fn load_theme_dirs(&self) -> Result<Vec<String>, String> {
        let config = self.read_config();
        match config
            .pointer("/appearance/theme_dirs")
            .and_then(Value::as_array)
        {
            Some(dirs) => {
                // La carpeta por defecto no va en la lista (versiones anteriores la guardaban ahí)
                let default_dir = self.default_dir_display();
                Ok(dirs
                    .iter()
                    .filter_map(|d| d.as_str().map(str::to_string))
                    .filter(|d| *d != default_dir)
                    .collect())
            }
            None => {
                // Primera vez: se deja el archivo con la lista vacía para que el usuario vea dónde va
                self.save_theme_dirs(&[])?;
                Ok(vec![])
            }
        }
    }

    fn save_theme_dirs(&self, dirs: &[String]) -> Result<Vec<String>, String> {
        let mut config = self.read_config();
        let root = config
            .as_object_mut()
            .expect("read_config devuelve un objeto");
        let appearance = root.entry("appearance").or_insert_with(|| json!({}));
        if !appearance.is_object() {
            *appearance = json!({});
        }
        appearance["theme_dirs"] = json!(dirs);
        self.write_config(&config)?;
        Ok(dirs.to_vec())
    }

    fn list_themes(&self) -> Result<Vec<GlobalTheme>, String> {
        let default_dir = self.default_dir_display();
        let _ = std::fs::create_dir_all(self.expand(&default_dir));
        let mut dirs = vec![default_dir];
        dirs.extend(self.load_theme_dirs()?);
        let mut themes: Vec<GlobalTheme> = Vec::new();
        for dir in dirs {
            let path = self.expand(&dir);
            for (id, file) in Self::css_files(&path) {
                if themes.iter().any(|t| t.id == id) {
                    continue;
                }
                if let Ok(css) = std::fs::read_to_string(&file) {
                    themes.push(GlobalTheme {
                        id,
                        css,
                        dir: dir.clone(),
                    });
                }
            }
        }
        Ok(themes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crea_config_y_lee_temas_de_varias_carpetas() {
        let home = tempfile::tempdir().unwrap();
        let config_dir = home.path().join(".config/synapsers");
        let repo = GlobalThemeRepositoryImpl::with_dirs(
            config_dir.clone(),
            Some(home.path().to_path_buf()),
        );

        // Primera lectura: crea config.jsonc con la lista vacía; la carpeta por defecto se lee igual
        assert_eq!(repo.default_theme_dir(), "~/.config/synapsers/themes");
        assert!(repo.load_theme_dirs().unwrap().is_empty());
        assert!(repo.list_themes().unwrap().is_empty());
        assert!(config_dir.join("themes").is_dir());

        std::fs::write(config_dir.join("themes/azul.css"), "a{}").unwrap();
        let otra = home.path().join("otra");
        std::fs::create_dir_all(&otra).unwrap();
        std::fs::write(otra.join("azul.css"), "repetido{}").unwrap();
        std::fs::write(otra.join("verde.css"), "v{}").unwrap();
        std::fs::write(otra.join("nota.md"), "x").unwrap();

        repo.save_theme_dirs(&[otra.to_string_lossy().into_owned()])
            .unwrap();
        let themes = repo.list_themes().unwrap();
        let ids: Vec<_> = themes
            .iter()
            .map(|t| (t.id.as_str(), t.css.as_str()))
            .collect();
        assert_eq!(ids, vec![("azul", "a{}"), ("verde", "v{}")]);

        // Conserva otras claves del archivo y lo deja legible como JSONC
        let raw = std::fs::read_to_string(config_dir.join("config.jsonc")).unwrap();
        assert!(raw.starts_with("// Configuración global"));
        let parsed: Value = serde_json_lenient::from_str(&raw).unwrap();
        assert_eq!(
            parsed["appearance"]["theme_dirs"][0],
            otra.to_string_lossy().as_ref()
        );
    }

    #[test]
    fn la_carpeta_por_defecto_guardada_antes_no_aparece_en_la_lista() {
        let home = tempfile::tempdir().unwrap();
        let config_dir = home.path().join(".config/synapsers");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.jsonc"),
            r#"{ "appearance": { "theme_dirs": ["~/.config/synapsers/themes", "/extra"] } }"#,
        )
        .unwrap();
        let repo =
            GlobalThemeRepositoryImpl::with_dirs(config_dir, Some(home.path().to_path_buf()));
        assert_eq!(repo.load_theme_dirs().unwrap(), vec!["/extra"]);
    }

    #[test]
    fn save_conserva_otras_claves() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.jsonc"),
            "// x\n{ \"otra\": 1, \"appearance\": { \"b\": 2 } }",
        )
        .unwrap();
        let repo = GlobalThemeRepositoryImpl::with_dirs(dir.path().to_path_buf(), None);
        repo.save_theme_dirs(&["/t".into()]).unwrap();
        let parsed: Value = serde_json_lenient::from_str(
            &std::fs::read_to_string(dir.path().join("config.jsonc")).unwrap(),
        )
        .unwrap();
        assert_eq!(parsed["otra"], 1);
        assert_eq!(parsed["appearance"]["b"], 2);
        assert_eq!(repo.load_theme_dirs().unwrap(), vec!["/t"]);
    }
}
