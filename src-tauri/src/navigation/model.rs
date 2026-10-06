use compact_str::CompactString;
use serde::{Deserialize, Serialize};

pub type NoteId = u32;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct NoteMeta {
    pub id: NoteId,
    pub path: CompactString,  // Ruta relativa normalizada
    pub title: CompactString, // Título derivado del H1 o nombre de archivo
    pub mtime_nanos: u128,    // Timestamp de modificación en nanosegundos
    pub size_bytes: u64,      // Tamaño en bytes para detección rápida de cambios
}
