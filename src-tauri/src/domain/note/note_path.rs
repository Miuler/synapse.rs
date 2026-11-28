use serde::{Deserialize, Serialize};

/// Value Object para representar una ruta relativa válida y segura dentro de la bóveda (Vault).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteRelativePath(String);

impl NoteRelativePath {
    pub fn new(path: &str) -> Result<Self, String> {
        // Prevención de seguridad: no permitir navegación fuera de la bóveda
        if path.contains("..") {
            return Err("path outside the vault".to_string());
        }

        // Normalización de la ruta
        let normalized = path
            .trim_start_matches('/')
            .trim_start_matches('\\')
            .replace('\\', "/");

        if normalized.is_empty() {
            return Err("path cannot be empty".to_string());
        }

        Ok(Self(normalized))
    }

    /// Crea una ruta a partir de una ubicación absoluta en el sistema de archivos (Modo puntual).
    pub fn from_absolute_path(path: &str) -> Result<Self, String> {
        if path.contains("..") {
            return Err("path with '..' is not allowed".to_string());
        }
        let normalized = path.replace('\\', "/");
        if normalized.is_empty() {
            return Err("path cannot be empty".to_string());
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
