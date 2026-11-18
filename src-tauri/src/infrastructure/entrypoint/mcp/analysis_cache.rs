//! Caché en memoria de archivos ya analizados para las herramientas del MCP (p. ej. un modelo
//! ArchiMate recorrido una vez y consultado muchas).
//!
//! - Clave: ruta, fecha de modificación, tamaño y tipo de análisis; si el archivo cambia, la clave
//!   ya no coincide y se vuelve a analizar (la entrada vieja sale por desuso).
//! - Límite por memoria total, no por cantidad: se descarta lo menos usado.
//! - Solo guarda lo costoso (tardó más de `MIN_COST` o el archivo pesa más de `MIN_FILE_BYTES`);
//!   lo barato se recalcula.

use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

/// Memoria total de la caché del proceso.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
const MIN_COST: Duration = Duration::from_millis(30);
const MIN_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    path: PathBuf,
    mtime: Option<SystemTime>,
    size: u64,
    kind: &'static str,
}

struct Entry {
    value: Arc<dyn Any + Send + Sync>,
    bytes: usize,
    last_used: u64,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<Key, Entry>,
    used: usize,
    tick: u64,
}

pub struct AnalysisCache {
    inner: Mutex<Inner>,
    max_bytes: usize,
}

/// Resultado de un análisis: el valor y la memoria aproximada que ocupa.
pub struct Analyzed<T> {
    pub value: T,
    pub bytes: usize,
}

impl AnalysisCache {
    pub fn new(max_bytes: usize) -> Self {
        Self { inner: Mutex::default(), max_bytes }
    }

    /// Caché compartida del proceso.
    pub fn global() -> &'static AnalysisCache {
        static CACHE: OnceLock<AnalysisCache> = OnceLock::new();
        CACHE.get_or_init(|| AnalysisCache::new(MAX_BYTES))
    }

    /// El análisis `kind` de `path`: el guardado si el archivo no cambió, o el de `analyze`.
    pub fn get_or_analyze<T, E>(
        &self,
        path: &Path,
        kind: &'static str,
        analyze: impl FnOnce() -> Result<Analyzed<T>, E>,
    ) -> Result<Arc<T>, E>
    where
        T: Any + Send + Sync,
    {
        let meta = std::fs::metadata(path).ok();
        let key = Key {
            path: path.to_path_buf(),
            mtime: meta.as_ref().and_then(|m| m.modified().ok()),
            size: meta.as_ref().map_or(0, |m| m.len()),
            kind,
        };
        if let Ok(mut inner) = self.inner.lock() {
            inner.tick += 1;
            let tick = inner.tick;
            if let Some(entry) = inner.entries.get_mut(&key) {
                entry.last_used = tick;
                if let Ok(value) = entry.value.clone().downcast::<T>() {
                    return Ok(value);
                }
            }
        }
        let started = Instant::now();
        let Analyzed { value, bytes } = analyze()?;
        let value = Arc::new(value);
        let costly = started.elapsed() >= MIN_COST || key.size >= MIN_FILE_BYTES;
        if costly && bytes <= self.max_bytes {
            self.insert(key, value.clone(), bytes);
        }
        Ok(value)
    }

    fn insert(&self, key: Key, value: Arc<dyn Any + Send + Sync>, bytes: usize) {
        let Ok(mut inner) = self.inner.lock() else { return };
        // Versiones viejas del mismo archivo y análisis: ya no sirven
        let stale: Vec<Key> = inner
            .entries
            .keys()
            .filter(|k| k.path == key.path && k.kind == key.kind && **k != key)
            .cloned()
            .collect();
        for k in stale {
            if let Some(e) = inner.entries.remove(&k) {
                inner.used -= e.bytes;
            }
        }
        // Descartar lo menos usado hasta que entre
        while inner.used + bytes > self.max_bytes {
            let Some(oldest) = inner.entries.iter().min_by_key(|(_, e)| e.last_used).map(|(k, _)| k.clone()) else {
                break;
            };
            if let Some(e) = inner.entries.remove(&oldest) {
                inner.used -= e.bytes;
            }
        }
        inner.tick += 1;
        let last_used = inner.tick;
        if let Some(old) = inner.entries.insert(key, Entry { value, bytes, last_used }) {
            inner.used -= old.bytes;
        }
        inner.used += bytes;
    }

    /// Entradas y memoria usada (para pruebas y diagnóstico).
    pub fn stats(&self) -> (usize, usize) {
        self.inner.lock().map(|i| (i.entries.len(), i.used)).unwrap_or((0, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Archivo de más de 1 MB (se guarda aunque el análisis sea rápido).
    fn big_file(dir: &Path, name: &str, fill: u8) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, vec![fill; 1024 * 1024 + 1]).unwrap();
        p
    }

    #[test]
    fn acierta_y_se_invalida_cuando_cambia_el_archivo() {
        let dir = tempfile::tempdir().unwrap();
        let path = big_file(dir.path(), "m.archimate", b'a');
        let cache = AnalysisCache::new(1024);
        let runs = Cell::new(0);
        let analyze = || -> Result<Analyzed<String>, ()> {
            runs.set(runs.get() + 1);
            Ok(Analyzed { value: format!("v{}", runs.get()), bytes: 100 })
        };
        assert_eq!(*cache.get_or_analyze(&path, "archimate", analyze).unwrap(), "v1");
        assert_eq!(*cache.get_or_analyze(&path, "archimate", analyze).unwrap(), "v1");
        assert_eq!(runs.get(), 1, "la segunda vez viene de la caché");
        // Otro tamaño: la clave cambia y se vuelve a analizar; la versión vieja se descarta
        std::fs::write(&path, vec![b'b'; 1024 * 1024 + 2]).unwrap();
        assert_eq!(*cache.get_or_analyze(&path, "archimate", analyze).unwrap(), "v2");
        assert_eq!(cache.stats(), (1, 100));
    }

    #[test]
    fn descarta_lo_menos_usado_al_pasar_el_limite_de_memoria() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, c) = (big_file(dir.path(), "a", 1), big_file(dir.path(), "b", 2), big_file(dir.path(), "c", 3));
        let cache = AnalysisCache::new(250);
        let put = |p: &Path, v: &'static str| {
            cache.get_or_analyze(p, "x", || Ok::<_, ()>(Analyzed { value: v, bytes: 100 })).unwrap()
        };
        put(&a, "a");
        put(&b, "b");
        put(&a, "a"); // a se usó después que b
        put(&c, "c"); // no entran tres: sale b
        assert_eq!(cache.stats(), (2, 200));
        let reanalyzed = Cell::new(false);
        cache
            .get_or_analyze(&b, "x", || {
                reanalyzed.set(true);
                Ok::<_, ()>(Analyzed { value: "b", bytes: 100 })
            })
            .unwrap();
        assert!(reanalyzed.get(), "b había salido de la caché");
    }

    #[test]
    fn lo_barato_no_se_guarda() {
        let dir = tempfile::tempdir().unwrap();
        let small = dir.path().join("chico.archimate");
        std::fs::write(&small, "<model/>").unwrap();
        let cache = AnalysisCache::new(1024);
        cache.get_or_analyze(&small, "archimate", || Ok::<_, ()>(Analyzed { value: 1, bytes: 10 })).unwrap();
        assert_eq!(cache.stats(), (0, 0));
    }
}
