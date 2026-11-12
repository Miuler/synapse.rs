use crate::domain::appearance::global_theme::GlobalTheme;

/// Carpetas de temas globales (configuradas en el `config.jsonc` del usuario) y los temas que hay en ellas.
pub trait GlobalThemeRepository: Send + Sync {
    /// Carpeta que se lee siempre (`~/.config/synapsers/themes`); no forma parte de la lista editable.
    fn default_theme_dir(&self) -> String;
    /// Carpetas extra que agregó el usuario.
    fn load_theme_dirs(&self) -> Result<Vec<String>, String>;
    fn save_theme_dirs(&self, dirs: &[String]) -> Result<Vec<String>, String>;
    /// Lee la carpeta por defecto y luego las extra; si dos tienen un tema con el mismo nombre gana la primera.
    fn list_themes(&self) -> Result<Vec<GlobalTheme>, String>;
}
