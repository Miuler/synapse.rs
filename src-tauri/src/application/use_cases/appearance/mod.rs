use crate::application::dto::appearance::{
    GetThemeDirsQuery, ListGlobalThemesQuery, SaveThemeDirsCommand,
};
use crate::domain::appearance::{GlobalTheme, GlobalThemeRepository};

/// Carpeta de temas globales que se lee siempre.
#[derive(Clone)]
pub struct GetDefaultThemeDirUseCase<R: GlobalThemeRepository> {
    repository: R,
}

impl<R: GlobalThemeRepository> GetDefaultThemeDirUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&self) -> String {
        self.repository.default_theme_dir()
    }
}

/// Carpetas extra de temas globales configuradas.
#[derive(Clone)]
pub struct GetThemeDirsUseCase<R: GlobalThemeRepository> {
    repository: R,
}

impl<R: GlobalThemeRepository> GetThemeDirsUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&self, _query: GetThemeDirsQuery) -> Result<Vec<String>, String> {
        self.repository.load_theme_dirs()
    }
}

/// Guarda las carpetas extra de temas globales, sin vacías, repetidas ni la carpeta por defecto.
#[derive(Clone)]
pub struct SaveThemeDirsUseCase<R: GlobalThemeRepository> {
    repository: R,
}

impl<R: GlobalThemeRepository> SaveThemeDirsUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&self, command: SaveThemeDirsCommand) -> Result<Vec<String>, String> {
        let default_dir = self.repository.default_theme_dir();
        let mut dirs: Vec<String> = Vec::new();
        for dir in command.dirs {
            let dir = dir.trim().to_string();
            if !dir.is_empty() && dir != default_dir && !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        self.repository.save_theme_dirs(&dirs)
    }
}

/// Temas que hay ahora en las carpetas de temas globales.
#[derive(Clone)]
pub struct ListGlobalThemesUseCase<R: GlobalThemeRepository> {
    repository: R,
}

impl<R: GlobalThemeRepository> ListGlobalThemesUseCase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&self, _query: ListGlobalThemesQuery) -> Result<Vec<GlobalTheme>, String> {
        self.repository.list_themes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fake(Mutex<Vec<String>>);

    impl GlobalThemeRepository for &Fake {
        fn default_theme_dir(&self) -> String {
            "~/.config/synapsers/themes".into()
        }
        fn load_theme_dirs(&self) -> Result<Vec<String>, String> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn save_theme_dirs(&self, dirs: &[String]) -> Result<Vec<String>, String> {
            *self.0.lock().unwrap() = dirs.to_vec();
            Ok(dirs.to_vec())
        }
        fn list_themes(&self) -> Result<Vec<GlobalTheme>, String> {
            Ok(vec![])
        }
    }

    #[test]
    fn save_quita_vacias_repetidas_y_la_por_defecto() {
        let fake = Fake::default();
        let saved = SaveThemeDirsUseCase::new(&fake)
            .execute(SaveThemeDirsCommand {
                dirs: vec![
                    " ~/a ".into(),
                    "".into(),
                    "~/.config/synapsers/themes".into(),
                    "~/a".into(),
                    "/b".into(),
                ],
            })
            .unwrap();
        assert_eq!(saved, vec!["~/a", "/b"]);
        assert_eq!(
            GetThemeDirsUseCase::new(&fake)
                .execute(GetThemeDirsQuery {})
                .unwrap(),
            saved
        );
    }
}
