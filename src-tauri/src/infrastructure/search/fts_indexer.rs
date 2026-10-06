use crate::domain::events::vault_events::{VaultChange, VaultChangeObserver};
use crate::domain::models::file_types::SupportedFileTypes;
use crate::domain::models::full_text::{FullTextDocument, FullTextIndexStatus};
use crate::domain::services::full_text_index::FullTextIndex;
use crate::infrastructure::search::content_extractor;
use crate::infrastructure::search::tantivy_index::TantivyFullTextIndex;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

enum IndexerCmd {
    Change(VaultChange),
    FullSync,
    Rebuild,
    Shutdown,
}

pub struct FtsIndexer {
    tx: mpsc::Sender<IndexerCmd>,
    worker_handle: Mutex<Option<thread::JoinHandle<()>>>,
    index: Arc<TantivyFullTextIndex>,
    pending_count: Arc<AtomicUsize>,
    is_indexing: Arc<AtomicBool>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl FtsIndexer {
    pub fn start(
        vault_path: PathBuf,
        file_types: SupportedFileTypes,
        index: Arc<TantivyFullTextIndex>,
    ) -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<IndexerCmd>();

        let pending_count = Arc::new(AtomicUsize::new(0));
        let is_indexing = Arc::new(AtomicBool::new(false));
        let last_error = Arc::new(Mutex::new(None));

        let worker_vault_path = vault_path;
        let worker_file_types = file_types;
        let worker_index = Arc::clone(&index);
        let worker_pending = Arc::clone(&pending_count);
        let worker_is_indexing = Arc::clone(&is_indexing);
        let worker_last_error = Arc::clone(&last_error);

        let handle = thread::Builder::new()
            .name("fts-indexer".to_string())
            .spawn(move || {
                let mut pending_upserts = HashSet::new();
                let mut pending_removes = HashSet::new();
                let mut last_activity = Instant::now();
                let debounce_dur = Duration::from_millis(350);

                let process_batch = |upserts: &mut HashSet<String>,
                                     removes: &mut HashSet<String>,
                                     vault: &Path,
                                     ft: &SupportedFileTypes,
                                     idx: &TantivyFullTextIndex,
                                     err_lock: &Arc<Mutex<Option<String>>>| {
                    if upserts.is_empty() && removes.is_empty() {
                        return;
                    }

                    // 1. Procesar eliminaciones
                    if !removes.is_empty() {
                        let to_remove: Vec<String> = removes.drain().collect();
                        // Si la ruta eliminada es una carpeta, eliminar todas las notas hijas
                        let mut final_deletions = Vec::new();
                        if let Ok(indexed) = idx.indexed_versions() {
                            for del in &to_remove {
                                final_deletions.push(del.clone());
                                let prefix = format!("{}/", del.trim_end_matches('/'));
                                for path in indexed.keys() {
                                    if path.starts_with(&prefix) {
                                        final_deletions.push(path.clone());
                                    }
                                }
                            }
                        } else {
                            final_deletions = to_remove;
                        }

                        if let Err(e) = idx.delete(&final_deletions) {
                            if let Ok(mut l) = err_lock.lock() {
                                *l = Some(format!("Error en delete FTS: {}", e));
                            }
                        }
                    }

                    // 2. Procesar inserciones / modificaciones concurrentemente
                    if !upserts.is_empty() {
                        let targets: Vec<String> = upserts.drain().collect();
                        let docs: Vec<FullTextDocument> = targets
                            .into_par_iter()
                            .filter_map(|rel_str| {
                                let abs = vault.join(&rel_str);
                                let meta = fs::metadata(&abs).ok()?;
                                if !meta.is_file() {
                                    return None;
                                }

                                let bytes = fs::read(&abs).ok()?;
                                let (kind, title, body) =
                                    content_extractor::extract(Path::new(&rel_str), &bytes, ft)?;

                                let mtime_nanos = meta
                                    .modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                                    .map(|d| d.as_nanos() as u64)
                                    .unwrap_or(0);

                                Some(FullTextDocument {
                                    path: rel_str,
                                    title,
                                    body,
                                    kind,
                                    mtime_nanos,
                                })
                            })
                            .collect();

                        if !docs.is_empty() {
                            if let Err(e) = idx.upsert(docs) {
                                if let Ok(mut l) = err_lock.lock() {
                                    *l = Some(format!("Error en upsert FTS: {}", e));
                                }
                            }
                        }
                    }

                    // 3. Commit unificado
                    if let Err(e) = idx.commit() {
                        if let Ok(mut l) = err_lock.lock() {
                            *l = Some(format!("Error en commit FTS: {}", e));
                        }
                    }
                };

                loop {
                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(IndexerCmd::Change(change)) => {
                            worker_is_indexing.store(true, Ordering::SeqCst);
                            match change {
                                VaultChange::Upserted(paths) => {
                                    for p in paths {
                                        pending_upserts.insert(p);
                                    }
                                }
                                VaultChange::Removed(paths) => {
                                    for p in paths {
                                        pending_upserts.remove(&p);
                                        pending_removes.insert(p);
                                    }
                                }
                            }
                            last_activity = Instant::now();
                            worker_pending
                                .store(pending_upserts.len() + pending_removes.len(), Ordering::SeqCst);
                        }
                        Ok(IndexerCmd::FullSync) => {
                            worker_is_indexing.store(true, Ordering::SeqCst);
                            perform_full_sync(
                                &worker_vault_path,
                                &worker_file_types,
                                &worker_index,
                                &worker_last_error,
                            );
                            worker_is_indexing.store(false, Ordering::SeqCst);
                            worker_pending.store(0, Ordering::SeqCst);
                        }
                        Ok(IndexerCmd::Rebuild) => {
                            worker_is_indexing.store(true, Ordering::SeqCst);
                            let _ = worker_index.delete_all();
                            let _ = worker_index.commit();
                            perform_full_sync(
                                &worker_vault_path,
                                &worker_file_types,
                                &worker_index,
                                &worker_last_error,
                            );
                            worker_is_indexing.store(false, Ordering::SeqCst);
                            worker_pending.store(0, Ordering::SeqCst);
                        }
                        Ok(IndexerCmd::Shutdown) => {
                            // Flush final antes de salir
                            process_batch(
                                &mut pending_upserts,
                                &mut pending_removes,
                                &worker_vault_path,
                                &worker_file_types,
                                &worker_index,
                                &worker_last_error,
                            );
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if (!pending_upserts.is_empty() || !pending_removes.is_empty())
                                && last_activity.elapsed() >= debounce_dur
                            {
                                process_batch(
                                    &mut pending_upserts,
                                    &mut pending_removes,
                                    &worker_vault_path,
                                    &worker_file_types,
                                    &worker_index,
                                    &worker_last_error,
                                );
                                worker_is_indexing.store(false, Ordering::SeqCst);
                                worker_pending.store(0, Ordering::SeqCst);
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
            })
            .expect("Fallo al iniciar hilo fts-indexer");

        let indexer = Arc::new(Self {
            tx,
            worker_handle: Mutex::new(Some(handle)),
            index,
            pending_count,
            is_indexing,
            last_error,
        });

        // Disparar sincronización inicial en segundo plano
        let _ = indexer.tx.send(IndexerCmd::FullSync);

        indexer
    }

    pub fn status(&self) -> FullTextIndexStatus {
        FullTextIndexStatus {
            indexed_docs: self.index.num_docs(),
            pending: self.pending_count.load(Ordering::SeqCst),
            is_indexing: self.is_indexing.load(Ordering::SeqCst),
            in_memory_fallback: self.index.is_in_memory(),
            last_error: self.last_error.lock().ok().and_then(|l| l.clone()),
        }
    }

    pub fn rebuild(&self) -> Result<(), String> {
        self.tx
            .send(IndexerCmd::Rebuild)
            .map_err(|e| format!("Error solicitando reconstrucción de índice: {}", e))
    }
}

impl VaultChangeObserver for FtsIndexer {
    fn on_vault_change(&self, change: VaultChange) {
        let _ = self.tx.send(IndexerCmd::Change(change));
    }
}

impl Drop for FtsIndexer {
    fn drop(&mut self) {
        let _ = self.tx.send(IndexerCmd::Shutdown);
        if let Ok(mut lock) = self.worker_handle.lock() {
            if let Some(handle) = lock.take() {
                let _ = handle.join();
            }
        }
    }
}

fn perform_full_sync(
    vault_path: &Path,
    file_types: &SupportedFileTypes,
    index: &TantivyFullTextIndex,
    error_lock: &Arc<Mutex<Option<String>>>,
) {
    let indexed = match index.indexed_versions() {
        Ok(v) => v,
        Err(e) => {
            if let Ok(mut l) = error_lock.lock() {
                *l = Some(format!("Error leyendo versiones indexadas: {}", e));
            }
            return;
        }
    };

    let mut seen_paths = HashSet::new();
    let mut to_index_paths = Vec::new();

    for entry_res in jwalk::WalkDir::new(vault_path).skip_hidden(true) {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type.is_file() {
            continue;
        }

        let abs_path = entry.path();
        let file_name = match abs_path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if file_name.starts_with('.') || !file_types.is_supported_file(file_name) {
            continue;
        }

        let rel_path_str = match abs_path.strip_prefix(vault_path) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let mtime_nanos: u64 = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        seen_paths.insert(rel_path_str.clone());

        let needs_indexing = match indexed.get(&rel_path_str) {
            Some(&existing_mtime) => existing_mtime != mtime_nanos,
            None => true,
        };

        if needs_indexing {
            to_index_paths.push((rel_path_str, mtime_nanos));
        }
    }

    // 1. Eliminar documentos que ya no existen en el disco
    let to_delete: Vec<String> = indexed
        .keys()
        .filter(|p| !seen_paths.contains(*p))
        .cloned()
        .collect();

    if !to_delete.is_empty() {
        if let Err(e) = index.delete(&to_delete) {
            if let Ok(mut l) = error_lock.lock() {
                *l = Some(format!("Error en delete de sync: {}", e));
            }
        }
    }

    // 2. Indexar archivos nuevos o modificados en paralelo con rayon
    if !to_index_paths.is_empty() {
        let docs: Vec<FullTextDocument> = to_index_paths
            .into_par_iter()
            .filter_map(|(rel_str, mtime_nanos)| {
                let abs = vault_path.join(&rel_str);
                let bytes = fs::read(&abs).ok()?;
                let (kind, title, body) =
                    content_extractor::extract(Path::new(&rel_str), &bytes, file_types)?;

                Some(FullTextDocument {
                    path: rel_str,
                    title,
                    body,
                    kind,
                    mtime_nanos,
                })
            })
            .collect();

        if !docs.is_empty() {
            if let Err(e) = index.upsert(docs) {
                if let Ok(mut l) = error_lock.lock() {
                    *l = Some(format!("Error en upsert de sync: {}", e));
                }
            }
        }
    }

    // 3. Commit
    if !to_delete.is_empty() || seen_paths.len() != indexed.len() {
        let _ = index.commit();
    }
}
