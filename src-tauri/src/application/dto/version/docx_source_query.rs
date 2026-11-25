/// De dónde sale una versión de un documento de Word.
pub enum DocxSource {
    Disk,
    Staged,
    Revision(String),
}

pub struct DocxSourceQuery {
    pub path: String,
    pub source: DocxSource,
}
