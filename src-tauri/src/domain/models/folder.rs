use serde::{Deserialize, Serialize};

/// Información y huella semántica de una carpeta descubierta en la bóveda.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FolderMeta {
    /// Ruta relativa normalizada de la carpeta (separador '/', sin barras iniciales ni finales).
    pub relative_path: String,
    /// Nombre visible del directorio (último segmento).
    pub name: String,
    /// Ruta de la carpeta padre si no es la raíz.
    #[serde(default)]
    pub parent: Option<String>,
    /// Cantidad total de archivos/notas contenidos directa e indirectamente.
    pub notes_count: usize,
    /// Cantidad de subcarpetas inmediatas.
    pub subfolders_count: usize,
    /// Etiquetas más representativas calculadas a partir de las notas de la carpeta.
    #[serde(default)]
    pub dominant_tags: Vec<String>,
    /// Nombres de archivo o títulos de muestra para desambiguación contextual instantánea.
    #[serde(default)]
    pub sample_notes: Vec<String>,
}

/// Resultado de una coincidencia de búsqueda difusa sobre carpetas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderSearchResult {
    pub folder: FolderMeta,
    pub score: u32,
    pub match_indices: Vec<u32>,
}
