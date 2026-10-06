use super::model::{NoteId, NoteMeta};
use dashmap::DashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
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
    bincode::serde::encode_into_std_write(map, &mut writer, bincode::config::standard())?;
    Ok(())
}

pub fn load_cache_from_disk(
    cache_path: &Path,
) -> Result<DashMap<NoteId, NoteMeta>, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(cache_path)?;
    let mut reader = BufReader::new(file);
    let map: DashMap<NoteId, NoteMeta> =
        bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())?;
    Ok(map)
}
