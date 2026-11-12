use serde::Serialize;

/// Tema CSS que el usuario guarda fuera de las bóvedas, en una de las carpetas de temas globales.
/// Vale para todas las bóvedas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalTheme {
    /// Nombre del archivo sin `.css`.
    pub id: String,
    pub css: String,
    /// Carpeta de donde salió, tal como está en la configuración.
    pub dir: String,
}
