use compact_str::CompactString;
use serde::{Deserialize, Serialize};

pub type NoteId = u32;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct NoteMeta {
    pub id: NoteId,
    pub path: CompactString,             // Ruta relativa normalizada
    pub title: CompactString,            // Título derivado del H1 o nombre de archivo
    pub mtime_nanos: u128,               // Timestamp de modificación en nanosegundos (cuándo se editó)
    pub size_bytes: u64,                 // Tamaño en bytes para detección rápida de cambios
    #[serde(default)]
    pub created_nanos: Option<u128>,     // Timestamp de creación (cuándo se creó en el filesystem)
    #[serde(default)]
    pub last_opened_nanos: Option<u128>, // Timestamp de cuándo se abrió en un tab para ver
    #[serde(default)]
    pub is_open: bool,                   // Si el archivo estaba abierto en una pestaña (open tab)
    #[serde(default)]
    pub tab_order: Option<u32>,          // Orden de la pestaña abierta
    #[serde(default)]
    pub is_active_tab: bool,             // Si era la pestaña activa seleccionada en pantalla
    #[serde(default)]
    pub view_mode: Option<CompactString>, // Modo del archivo ("reading", "live", "source")
}
