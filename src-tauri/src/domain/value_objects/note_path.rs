use serde::{Deserialize, Serialize};

/// Value Object para representar una ruta relativa válida y segura dentro de la bóveda (Vault).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteRelativePath(String);

impl NoteRelativePath {
    pub fn new(path: &str) -> Result<Self, String> {
        // Prevención de seguridad: no permitir navegación fuera de la bóveda
        if path.contains("..") {
            return Err("Ruta no válida: no se permite navegación fuera de la bóveda (Path Traversal)".to_string());
        }

        // Normalización de la ruta
        let normalized = path
            .trim_start_matches('/')
            .trim_start_matches('\\')
            .replace('\\', "/");

        if normalized.is_empty() {
            return Err("La ruta no puede estar vacía".to_string());
        }

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
