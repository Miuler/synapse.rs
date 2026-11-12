use super::model::{NoteId, NoteMeta};
use dashmap::DashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn save_cache_to_disk(
    cache_path: &Path,
    map: &DashMap<NoteId, NoteMeta>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(cache_path)?;
    let mut writer = BufWriter::new(file);
    postcard::to_io(map, &mut writer)?;
    Ok(())
}

pub fn load_cache_from_disk(
    cache_path: &Path,
) -> Result<DashMap<NoteId, NoteMeta>, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = std::fs::read(cache_path)?;
    let map: DashMap<NoteId, NoteMeta> = postcard::from_bytes(&bytes)?;
    Ok(map)
}
