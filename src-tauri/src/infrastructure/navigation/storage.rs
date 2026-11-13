//! Persistencia de la caché de metadatos de notas (`.synapse/cache.bin`).
//!
//! `postcard` convierte el `DashMap` en bytes y `object_store` los guarda. Con el backend local,
//! `put` escribe en un archivo temporal y luego lo renombra: si la app se cae a mitad de un
//! guardado, `cache.bin` conserva la versión anterior en lugar de quedar corrupto. Cambiar el
//! destino (otro backend de `object_store`) no requiere tocar el resto del motor.
use super::model::{NoteId, NoteMeta};
use dashmap::DashMap;
use object_store::local::LocalFileSystem;
use object_store::path::Path as StorePath;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};
use std::path::Path;
use std::sync::Arc;

type CacheResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Abre el almacén local en la carpeta de `cache_path` y devuelve la clave del archivo.
fn open_store(cache_path: &Path) -> CacheResult<(Arc<dyn ObjectStore>, StorePath)> {
    let dir = cache_path
        .parent()
        .ok_or("la ruta de la caché no tiene carpeta")?;
    std::fs::create_dir_all(dir)?;
    let name = cache_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("la ruta de la caché no tiene nombre de archivo")?;
    let store = LocalFileSystem::new_with_prefix(dir)?;
    Ok((Arc::new(store), StorePath::from(name)))
}

// El motor guarda desde hilos `std` (debounce) y carga en el arranque, fuera de un runtime async.
// `LocalFileSystem` hace su E/S de forma síncrona cuando no hay runtime de tokio, y la delega a
// `spawn_blocking` cuando lo hay, así que `block_on` es seguro en ambos casos.
pub fn save_cache_to_disk(cache_path: &Path, map: &DashMap<NoteId, NoteMeta>) -> CacheResult<()> {
    let (store, key) = open_store(cache_path)?;
    let bytes = postcard::to_allocvec(map)?;
    futures_executor::block_on(store.put(&key, PutPayload::from(bytes)))?;
    Ok(())
}

pub fn load_cache_from_disk(cache_path: &Path) -> CacheResult<DashMap<NoteId, NoteMeta>> {
    let (store, key) = open_store(cache_path)?;
    let bytes = futures_executor::block_on(async { store.get(&key).await?.bytes().await })?;
    let map: DashMap<NoteId, NoteMeta> = postcard::from_bytes(&bytes)?;
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use compact_str::CompactString;

    fn sample_map() -> DashMap<NoteId, NoteMeta> {
        let map = DashMap::new();
        let meta = NoteMeta {
            id: 1,
            path: CompactString::new("notas/a.md"),
            title: CompactString::new("a"),
            tags: vec![],
            mtime_nanos: 1,
            size_bytes: 10,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
            sync_state: Default::default(),
        };
        map.insert(1, meta);
        map
    }

    #[test]
    fn guarda_y_carga_la_cache() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".synapse").join("cache.bin");
        save_cache_to_disk(&path, &sample_map()).unwrap();
        let loaded = load_cache_from_disk(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().path, "notas/a.md");
    }

    #[test]
    fn guarda_dentro_de_un_runtime_tokio() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.bin");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let p = path.clone();
            tokio::task::spawn_blocking(move || save_cache_to_disk(&p, &sample_map()).unwrap())
                .await
                .unwrap();
        });
        assert_eq!(load_cache_from_disk(&path).unwrap().len(), 1);
    }

    #[test]
    fn no_deja_archivos_temporales() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.bin");
        save_cache_to_disk(&path, &sample_map()).unwrap();
        save_cache_to_disk(&path, &sample_map()).unwrap();
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec!["cache.bin"]);
    }
}
